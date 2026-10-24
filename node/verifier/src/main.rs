mod api;
mod attacks;
mod chain;
mod store;

use alloy::{
    primitives::{Address, Bytes, keccak256},
    signers::local::PrivateKeySigner,
    sol_types::Eip712Domain,
};
use anyhow::Result;
use chain::Chain;
use std::{collections::HashMap, env, sync::Mutex};
use store::{CertRow, Store};
use sunmint_core::{
    CHAIN_ID, SignedClaim, attest,
    checks::{Report, Site, Stage, StageName, Status, check_claim, short},
    demo::{self, GEN_FACTOR, grid_factor},
    domain, sign,
};

pub struct App {
    pub store: Mutex<Store>,
    pub chain: Chain,
    pub domain: Eip712Domain,
    pub chain_id: u64,
    pub certs_addr: Address,
    pub verifier: PrivateKeySigner,
    pub sites: HashMap<Address, Site>,
    pub mint_lock: tokio::sync::Mutex<()>,
}

pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

pub fn price_per_kwh(factor: u32) -> u64 {
    60_000 + factor as u64 * 50
}

impl App {
    /// Off-chain checks, co-signature, mint, market listing. One mint at a time.
    pub async fn process_claim(&self, sc: &SignedClaim) -> Report {
        let _guard = self.mint_lock.lock().await;
        let c = &sc.claim;
        let site = self.sites.get(&c.device);
        let readings: Vec<_> = self.store.lock().unwrap().hour_readings(c.device, c.hour).into_iter().map(|(r, _)| r).collect();
        let mut rep = check_claim(sc, site, &readings, now(), &self.domain);
        if !rep.passed() {
            self.store.lock().unwrap().reject(c.device, "claim", c.hour as u64 * 3600, &format!("{:?}", rep.verdict), &rep.reason, now());
            return rep;
        }
        let site = site.expect("checked");
        let att = attest(c, &self.domain, &self.verifier);
        let is_gen = c.gridDown && site.has_generator;
        let factor = if is_gen { GEN_FACTOR } else { grid_factor(&site.country) };
        match self.chain.mint(c, Bytes::from(sc.sig.as_bytes().to_vec()), Bytes::from(att.as_bytes().to_vec())).await {
            Err(reason) => {
                set_last(&mut rep, Status::Reject, &reason);
                self.store.lock().unwrap().reject(c.device, "claim", c.hour as u64 * 3600, "Reject", &reason, now());
            }
            Ok(tx) => {
                let id = chain::certificate_id(c.device, c.hour);
                let price = price_per_kwh(factor);
                let listing = match self.chain.list(id, c.wh, price).await {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::warn!("listing failed: {e}");
                        None
                    }
                };
                let row = CertRow {
                    id: format!("{id:#x}"),
                    device: c.device.to_string(),
                    hour: c.hour,
                    wh: c.wh,
                    source: if is_gen { "GEN" } else { "GRID" }.into(),
                    factor,
                    g_co2: c.wh as u64 * factor as u64 / 1000,
                    root: c.readingsRoot.to_string(),
                    claim_sig: sc.sig.to_string(),
                    attest_sig: att.to_string(),
                    tx: tx.hash.to_string(),
                    block: tx.block,
                    minted_at: now(),
                    listing,
                    price_per_kwh: price,
                    sold_wh: 0,
                };
                let _ = self.store.lock().unwrap().insert_cert(&row);
                set_last(&mut rep, Status::Pass, &format!("Minted {} Wh {} in tx {}", c.wh, row.source, short_tx(&row.tx)));
                rep.reason = format!("Minted certificate for {} hour {}.", site.name, c.hour);
            }
        }
        rep
    }
}

pub fn set_last(rep: &mut Report, status: Status, detail: &str) {
    if let Some(s) = rep.stages.iter_mut().find(|s| s.name == StageName::OnChain) {
        *s = Stage { name: StageName::OnChain, status, detail: detail.into() };
    }
    rep.verdict = status;
    rep.reason = detail.into();
}

pub fn short_tx(h: &str) -> String {
    format!("{}…{}", &h[..6], &h[h.len() - 4..])
}

fn demo_key(label: &str) -> PrivateKeySigner {
    PrivateKeySigner::from_bytes(&keccak256(label)).unwrap()
}

fn key_env(name: &str, fallback: &str) -> PrivateKeySigner {
    env::var(name).ok().and_then(|k| k.parse().ok()).unwrap_or_else(|| demo_key(fallback))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter(env::var("RUST_LOG").unwrap_or_else(|_| "info".into())).init();

    let verifier = key_env("VERIFIER_KEY", "sunmint-dry-verifier");
    let company = key_env("COMPANY_KEY", "sunmint-dry-company");
    let chain_id = env::var("CHAIN_ID").ok().and_then(|v| v.parse().ok()).unwrap_or(CHAIN_ID);
    let addr = |k: &str| env::var(k).ok().and_then(|v| v.parse::<Address>().ok());
    let (chain, certs_addr) = match (env::var("RPC_URL").ok(), addr("CERTS_ADDRESS"), addr("MARKET_ADDRESS"), addr("USD_ADDRESS")) {
        (Some(rpc), Some(c), Some(m), Some(u)) => (Chain::live(&rpc, verifier.clone(), company, c, m, u)?, c),
        _ => {
            tracing::warn!("RPC_URL / contract addresses not set: running in dry mode (no chain)");
            (Chain::dry(), Address::ZERO)
        }
    };
    let dom = domain(certs_addr, chain_id);

    if env::args().nth(1).as_deref() == Some("setup") {
        println!("verifier/producer wallet {}", verifier.address());
        let regs = demo::SITES
            .iter()
            .map(|s| {
                let k = s.key();
                let reg = Register { owner: verifier.address(), capacityW: s.capacity_w, hasGenerator: s.has_generator, country: country_bytes(s.country).into() };
                let sig = sign(&reg, &dom, &k);
                (k.address(), s.capacity_w, s.has_generator, country_bytes(s.country), Bytes::from(sig.as_bytes().to_vec()))
            })
            .collect();
        chain.setup(regs).await?;
        println!("done");
        return Ok(());
    }

    let store = Store::open(&env::var("DB_PATH").unwrap_or_else(|_| "sunmint.db".into()))?;
    let chain = match chain {
        Chain::Dry { .. } => {
            let minted = store.certs_recent(100_000).into_iter().filter_map(|c| Some((c.device.parse().ok()?, c.hour))).collect();
            let receipts = store.retirements(None, 1).first().map_or(0, |r| r.receipt);
            Chain::Dry { minted: Mutex::new(minted), seq: Mutex::new(store.totals().2 + receipts), receipts: Mutex::new(receipts) }
        }
        live => live,
    };
    let sites = demo::SITES.iter().map(|s| (s.key().address(), s.site())).collect();
    let app = std::sync::Arc::new(App {
        store: Mutex::new(store),
        chain,
        domain: dom,
        chain_id,
        certs_addr,
        verifier,
        sites,
        mint_lock: Default::default(),
    });
    tracing::info!("verifier {} · live chain: {}", short(app.verifier.address()), app.chain.is_live());

    let port = env::var("PORT").unwrap_or_else(|_| "8080".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("listening on :{port}");
    axum::serve(listener, api::router(app)).await?;
    Ok(())
}

alloy::sol! {
    struct Register { address owner; uint32 capacityW; bool hasGenerator; bytes2 country; }
}

fn country_bytes(cc: &str) -> [u8; 2] {
    let b = cc.as_bytes();
    [b[0], b[1]]
}
