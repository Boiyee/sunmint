//! Demo meters. Each site signs 5-minute readings from a clear-sky model with clouds and its outage
//! schedule, posts them to the verifier, and signs an hour claim once each hour closes.
//! On start it backfills the last `--backfill` hours so the app has a day of history.

use alloy::{primitives::Address, signers::local::PrivateKeySigner};
use anyhow::Result;
use clap::Parser;
use serde_json::Value;
use std::collections::HashMap;
use sunmint_core::{
    Reading, SignedClaim, SignedReading,
    checks::honest_claim,
    demo::{DemoSite, SITES},
    domain, sign, solar,
};

#[derive(Parser)]
struct Args {
    #[arg(long, env = "VERIFIER_URL", default_value = "http://localhost:8080")]
    url: String,
    #[arg(long, default_value_t = 24)]
    backfill: u64,
    /// Exit after backfill instead of running live.
    #[arg(long)]
    once: bool,
}

const STEP: u64 = 300;

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

/// Cheap deterministic hash noise in [0, 1).
fn noise(seed: u64) -> f64 {
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 31;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 29;
    (x >> 11) as f64 / (1u64 << 53) as f64
}

/// Cloud cover factor for a site at a time: a slow day-level haze plus passing clouds.
fn clouds(site: usize, ts: u64) -> f64 {
    let day = ts / 86_400;
    let haze = 0.82 + 0.16 * noise(day * 31 + site as u64);
    let slot = ts / 900; // 15-minute cloud cells
    let a = noise(slot * 7 + site as u64 * 1013);
    let b = noise((slot + 1) * 7 + site as u64 * 1013);
    let frac = (ts % 900) as f64 / 900.0;
    let cell = a + (b - a) * frac;
    haze * if cell > 0.86 { 0.45 + 0.4 * (1.0 - cell) } else { 1.0 }
}

fn wh_for(i: usize, s: &DemoSite, ts: u64) -> u32 {
    let mid = ts as i64 - STEP as i64 / 2;
    let f = solar::clear_sky_fraction(solar::elevation(mid, s.lat, s.lon)) * clouds(i, ts);
    (s.capacity_w as f64 * f * STEP as f64 / 3600.0).round() as u32
}

fn grid_v(i: usize, s: &DemoSite, ts: u64) -> u16 {
    if s.grid_down_at(ts - 1) { 0 } else { 226 + (noise(ts * 3 + i as u64) * 9.0) as u16 }
}

struct Meter {
    i: usize,
    site: &'static DemoSite,
    key: PrivateKeySigner,
    nonce: u64,
    last_ts: u64,
    hour: Vec<Reading>,
}

async fn post(http: &reqwest::Client, url: &str, body: impl serde::Serialize) -> Result<Value> {
    Ok(http.post(url).json(&body).send().await?.error_for_status()?.json().await?)
}

impl Meter {
    async fn tick(&mut self, ts: u64, http: &reqwest::Client, base: &str, d: &alloy::sol_types::Eip712Domain) -> Result<()> {
        let hour = ((ts - 1) / 3600) as u32;
        if let Some(prev) = self.hour.first().map(|r| ((r.ts - 1) / 3600) as u32) {
            if prev != hour {
                self.claim(prev, http, base, d).await?;
            }
        }
        self.nonce += 1;
        let r = Reading { device: self.key.address(), ts, wh: wh_for(self.i, self.site, ts), gridV: grid_v(self.i, self.site, ts), nonce: self.nonce };
        let rep = post(http, &format!("{base}/v1/readings"), SignedReading { sig: sign(&r, d, &self.key), reading: r }).await?;
        if rep["verdict"] == "pass" {
            self.hour.push(r);
            self.last_ts = ts;
        } else {
            eprintln!("{} reading rejected: {}", self.site.slug, rep["reason"]);
        }
        Ok(())
    }

    async fn claim(&mut self, hour: u32, http: &reqwest::Client, base: &str, d: &alloy::sol_types::Eip712Domain) -> Result<()> {
        let rs = std::mem::take(&mut self.hour);
        if rs.iter().all(|r| r.wh == 0) {
            return Ok(()); // night: nothing to certify
        }
        let c = honest_claim(self.key.address(), hour, &rs);
        let rep = post(http, &format!("{base}/v1/claims"), SignedClaim { sig: sign(&c, d, &self.key), claim: c }).await?;
        println!(
            "{:<9} hour {hour} {:>6} Wh {:<4} → {} {}",
            self.site.slug,
            c.wh,
            if c.gridDown { "down" } else { "up" },
            rep["verdict"].as_str().unwrap_or("?"),
            rep["reason"].as_str().unwrap_or("")
        );
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let http = reqwest::Client::new();
    let base = args.url.trim_end_matches('/').to_owned();

    let cfg: Value = loop {
        match http.get(format!("{base}/v1/config")).send().await {
            Ok(r) => break r.json().await?,
            Err(e) => {
                eprintln!("waiting for verifier at {base}: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    };
    let certs: Address = cfg["certs"].as_str().unwrap_or_default().parse().unwrap_or_default();
    let d = domain(certs, cfg["chainId"].as_u64().unwrap_or(sunmint_core::CHAIN_ID));
    println!("verifier live chain: {} · contract {certs}", cfg["live"]);

    // Resume from what the verifier already accepted, so restarts don't trip replay protection.
    let mut meters: Vec<Meter> = Vec::new();
    for (i, s) in SITES.iter().enumerate() {
        let st: Value = http.get(format!("{base}/v1/sites/{}", s.slug)).send().await?.json().await?;
        let last = st["stream"][0].clone();
        meters.push(Meter {
            i,
            site: s,
            key: s.key(),
            nonce: last["nonce"].as_u64().unwrap_or(0),
            last_ts: last["ts"].as_u64().unwrap_or(0),
            hour: Vec::new(),
        });
    }

    let t = now();
    let start = (t - args.backfill * 3600) / STEP * STEP + STEP;
    let mut ts = meters.iter().map(|m| m.last_ts + STEP).min().unwrap_or(start).max(start);
    println!("backfilling from {ts} to {t} ({} steps)", (t.saturating_sub(ts)) / STEP);
    while ts <= t {
        for m in meters.iter_mut().filter(|m| m.last_ts < ts) {
            if let Err(e) = m.tick(ts, &http, &base, &d).await {
                eprintln!("{} tick failed: {e}", m.site.slug);
            }
        }
        ts += STEP;
    }
    if args.once {
        return Ok(());
    }

    println!("live: one reading per site every {STEP} s");
    let mut last_hour: HashMap<usize, u32> = HashMap::new();
    loop {
        let next = (now() / STEP + 1) * STEP;
        tokio::time::sleep(std::time::Duration::from_secs(next.saturating_sub(now()))).await;
        for m in meters.iter_mut() {
            if let Err(e) = m.tick(next, &http, &base, &d).await {
                eprintln!("{} tick failed: {e}", m.site.slug);
            }
            last_hour.insert(m.i, ((next - 1) / 3600) as u32);
        }
    }
}
