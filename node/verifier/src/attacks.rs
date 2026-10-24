//! The Break it attacks. Each one is signed with a demo meter's real key, as if the key were stolen,
//! and sent through the same pipeline as honest traffic. Rejected attacks never change device state.

use crate::{App, now, set_last};
use alloy::primitives::Bytes;
use serde::Serialize;
use sunmint_core::{
    Reading, SignedClaim, SignedReading, attest,
    checks::{Report, Status, check_claim, check_reading, honest_claim},
    demo::{self, DemoSite},
    sign, solar,
};

#[derive(Serialize)]
pub struct Field {
    k: &'static str,
    v: String,
    edited: bool,
}

#[derive(Serialize)]
pub struct Outcome {
    kind: &'static str,
    title: String,
    how: String,
    note: String,
    payload: Vec<Field>,
    report: Report,
    outcome: String,
}

fn f(k: &'static str, v: impl ToString, edited: bool) -> Field {
    Field { k, v: v.to_string(), edited }
}

fn reading_fields(r: &Reading, edited: &[&str]) -> Vec<Field> {
    let e = |k| edited.contains(&k);
    vec![
        f("device", r.device, false),
        f("ts", r.ts, e("ts")),
        f("wh", r.wh, e("wh")),
        f("gridV", r.gridV, e("gridV")),
        f("nonce", r.nonce, e("nonce")),
    ]
}

fn lagos() -> &'static DemoSite {
    demo::by_slug("lagos").unwrap()
}

/// The site where the sun is lowest (for the night attack) or highest (for the capacity attack).
fn site_by_sun(highest: bool) -> &'static DemoSite {
    let t = now() as i64;
    let mut v: Vec<_> = demo::SITES.iter().collect();
    v.sort_by(|a, b| solar::elevation(t, a.lat, a.lon).total_cmp(&solar::elevation(t, b.lat, b.lon)));
    if highest { v[v.len() - 1] } else { v[0] }
}

pub async fn run(app: &App, kind: &str) -> Option<Outcome> {
    let st = |s: &DemoSite| app.store.lock().unwrap().state(s.key().address());
    let check = |s: &DemoSite, sr: &SignedReading| {
        let site = app.sites.get(&s.key().address());
        check_reading(sr, site, &st(s), now(), &app.domain)
    };

    let out = match kind {
        "forge" => {
            let s = lagos();
            let (r, sig) = app.store.lock().unwrap().last_reading_sig(s.key().address())?;
            let forged = Reading { wh: r.wh.max(40) * 10, ..r };
            let sr = SignedReading { reading: forged, sig: sig.parse().ok()? };
            Outcome {
                kind: "forge",
                title: "Forge a signature".into(),
                how: format!("Take {}'s last signed reading, change {} Wh to {} Wh, resend with the old signature.", s.name, r.wh, forged.wh),
                note: "wh edited after signing".into(),
                payload: reading_fields(&forged, &["wh"]),
                report: check(s, &sr),
                outcome: "Dropped at the door. Nothing stored, nothing minted.".into(),
            }
        }
        "replay" => {
            let s = lagos();
            let (r, sig) = app.store.lock().unwrap().last_reading_sig(s.key().address())?;
            let sr = SignedReading { reading: r, sig: sig.parse().ok()? };
            Outcome {
                kind: "replay",
                title: "Replay a reading".into(),
                how: format!("Resend {}'s last valid reading, untouched, signature and all.", s.name),
                note: "valid signature, old message".into(),
                payload: reading_fields(&r, &["nonce", "ts"]),
                report: check(s, &sr),
                outcome: "The signature was fine. The freshness check stopped it.".into(),
            }
        }
        "night" => {
            let s = site_by_sun(false);
            let k = s.key();
            let d = st(s);
            let r = Reading { device: k.address(), ts: (d.last_ts + 300).min(now()), wh: 400, gridV: 230, nonce: d.last_nonce + 1 };
            let sr = SignedReading { sig: sign(&r, &app.domain, &k), reading: r };
            let elev = solar::elevation(r.ts as i64, s.lat, s.lon);
            Outcome {
                kind: "night",
                title: "Solar in the dark".into(),
                how: format!("Use {}'s stolen key to sign 400 Wh while the sun there is at {elev:.0}°.", s.name),
                note: "meter key used to sign a lie".into(),
                payload: reading_fields(&r, &["wh"]),
                report: check(s, &sr),
                outcome: "Even a stolen meter key cannot beat physics.".into(),
            }
        }
        "capacity" => {
            let s = site_by_sun(true);
            let k = s.key();
            let d = st(s);
            let wh = (s.capacity_w as f64 * 300.0 / 3600.0 * 1.6) as u32;
            let r = Reading { device: k.address(), ts: (d.last_ts + 300).min(now()), wh, gridV: 230, nonce: d.last_nonce + 1 };
            let sr = SignedReading { sig: sign(&r, &app.domain, &k), reading: r };
            Outcome {
                kind: "capacity",
                title: "Exceed capacity".into(),
                how: format!("Claim {wh} Wh in five minutes from {}'s {:.1} kWp array.", s.name, s.capacity_w as f64 / 1000.0),
                note: "output above nameplate".into(),
                payload: reading_fields(&r, &["wh"]),
                report: check(s, &sr),
                outcome: "Rejected. Repeat offenders get flagged for review.".into(),
            }
        }
        "blackout" => {
            let s = lagos();
            let k = s.key();
            let dev = k.address();
            let hour_now = (now() / 3600) as u32;
            let (hour, rs) = (1..48).find_map(|back| {
                let h = hour_now - back;
                let rs: Vec<_> = app.store.lock().unwrap().hour_readings(dev, h).into_iter().map(|(r, _)| r).collect();
                (rs.len() >= 6 && rs.iter().all(|r| r.gridV >= 50)).then_some((h, rs))
            })?;
            let mut c = honest_claim(dev, hour, &rs);
            c.gridDown = true;
            let sc = SignedClaim { sig: sign(&c, &app.domain, &k), claim: c };
            let report = check_claim(&sc, app.sites.get(&dev), &rs, now(), &app.domain);
            let kwh = c.wh as f64 / 1000.0;
            Outcome {
                kind: "blackout",
                title: "Fake a blackout".into(),
                how: format!("Claim {}'s generator was displaced in hour {hour} while its signed grid channel shows the grid was up.", s.name),
                note: "GEN claimed, readings say grid up".into(),
                payload: vec![f("device", dev, false), f("hour", hour, false), f("wh", c.wh, false), f("gridDown", true, true), f("readingsRoot", c.readingsRoot, false)],
                report,
                outcome: format!(
                    "Only the honest claim counts: {:.2} kg CO₂ at the grid factor, not {:.2} kg at diesel. The lie earns nothing.",
                    kwh * demo::grid_factor(s.country) as f64 / 1000.0,
                    kwh * demo::GEN_FACTOR as f64 / 1000.0
                ),
            }
        }
        "double" => {
            let s = lagos();
            let k = s.key();
            let dev = k.address();
            let cert = app.store.lock().unwrap().certs_of(dev, 0).pop()?;
            let rs: Vec<_> = app.store.lock().unwrap().hour_readings(dev, cert.hour).into_iter().map(|(r, _)| r).collect();
            let c = honest_claim(dev, cert.hour, &rs);
            let sc = SignedClaim { sig: sign(&c, &app.domain, &k), claim: c };
            let mut report = check_claim(&sc, app.sites.get(&dev), &rs, now(), &app.domain);
            if report.passed() {
                let att = attest(&c, &app.domain, &app.verifier);
                let r = {
                    let _g = app.mint_lock.lock().await;
                    app.chain.mint(&c, Bytes::from(sc.sig.as_bytes().to_vec()), Bytes::from(att.as_bytes().to_vec())).await
                };
                match r {
                    Err(reason) => set_last(&mut report, Status::Reject, &reason),
                    Ok(_) => set_last(&mut report, Status::Pass, "Minted twice. This should never happen."),
                }
            }
            Outcome {
                kind: "double",
                title: "Mint the same hour twice".into(),
                how: format!("Send a second, perfectly honest claim for {}'s hour {}, which is already minted.", s.name, cert.hour),
                note: "second claim for a minted hour".into(),
                payload: vec![f("device", dev, false), f("hour", cert.hour, true), f("wh", c.wh, false), f("gridDown", c.gridDown, false), f("readingsRoot", c.readingsRoot, false)],
                report,
                outcome: "Every off-chain check passed. The contract itself refused. No backend can override it.".into(),
            }
        }
        _ => return None,
    };
    if out.report.verdict != Status::Pass {
        let dev = demo::by_slug("lagos").map(|s| s.key().address()).unwrap_or_default();
        app.store.lock().unwrap().reject(dev, &format!("attack:{}", out.kind), now(), &format!("{:?}", out.report.verdict), &out.report.reason, now());
    }
    Some(out)
}
