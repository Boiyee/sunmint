//! HTTP API for meters (readings, claims) and for the web app (views, attacks, match, retire).

use crate::{App, attacks, now, store::{CertRow, RetirementRow}};
use alloy::primitives::Address;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use sunmint_core::{
    SignedClaim, SignedReading,
    checks::{GRID_DOWN_V, Site, check_reading, short},
    demo, leaf, merkle, solar,
};
use tower_http::cors::CorsLayer;

type A = State<Arc<App>>;

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/v1/config", get(config))
        .route("/v1/readings", post(post_reading))
        .route("/v1/claims", post(post_claim))
        .route("/v1/network", get(network))
        .route("/v1/sites/{slug}", get(site))
        .route("/v1/certs/{id}", get(cert))
        .route("/v1/market", get(market))
        .route("/v1/match", post(match_load))
        .route("/v1/retire", post(retire))
        .route("/v1/receipts/{n}", get(receipt))
        .route("/v1/attacks/{kind}", post(attack))
        .route("/healthz", get(|| async { "ok" }))
        .layer(CorsLayer::permissive())
        .with_state(app)
}

fn not_found(what: &str) -> Response {
    (StatusCode::NOT_FOUND, Json(json!({ "error": format!("{what} not found") }))).into_response()
}

async fn config(State(app): A) -> Json<Value> {
    let (certs, market, usd) = app.chain.addresses().map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string())).unwrap_or_default();
    Json(json!({
        "live": app.chain.is_live(),
        "chainId": app.chain_id,
        "certs": app.certs_addr,
        "market": market,
        "usd": usd,
        "certsAddress": certs,
        "verifier": app.verifier.address(),
        "explorer": "https://sepolia.basescan.org",
        "genFactor": demo::GEN_FACTOR,
    }))
}

async fn post_reading(State(app): A, Json(sr): Json<SignedReading>) -> Json<Value> {
    let dev = sr.reading.device;
    let store = app.store.lock().unwrap();
    let rep = check_reading(&sr, app.sites.get(&dev), &store.state(dev), now(), &app.domain);
    if rep.passed() {
        let _ = store.accept(&sr.reading, &sr.sig);
    } else {
        store.reject(dev, "reading", sr.reading.ts, &format!("{:?}", rep.verdict), &rep.reason, now());
    }
    Json(json!(rep))
}

async fn post_claim(State(app): A, Json(sc): Json<SignedClaim>) -> Json<Value> {
    Json(json!(app.process_claim(&sc).await))
}

#[derive(Serialize)]
struct CertView {
    id: String,
    short: String,
    slug: String,
    site: String,
    country: String,
    hour: u32,
    wh: u32,
    source: String,
    factor: u32,
    g_co2: u64,
    tx: String,
    block: u64,
    minted_at: u64,
    listing: Option<u64>,
    price_per_kwh: u64,
    available_wh: u32,
}

fn slug_of(app: &App, device: &str) -> (String, String, String) {
    device
        .parse::<Address>()
        .ok()
        .and_then(|d| demo::SITES.iter().find(|s| s.key().address() == d))
        .map(|s| (s.slug.to_string(), s.name.to_string(), s.country.to_string()))
        .or_else(|| {
            let d: Address = device.parse().ok()?;
            app.sites.get(&d).map(|s| (short(d), s.name.clone(), s.country.clone()))
        })
        .unwrap_or_default()
}

fn view(app: &App, c: &CertRow) -> CertView {
    let (slug, site, country) = slug_of(app, &c.device);
    CertView {
        short: format!("#{}", &c.id[c.id.len() - 4..].to_uppercase()),
        id: c.id.clone(),
        slug,
        site,
        country,
        hour: c.hour,
        wh: c.wh,
        source: c.source.clone(),
        factor: c.factor,
        g_co2: c.g_co2,
        tx: c.tx.clone(),
        block: c.block,
        minted_at: c.minted_at,
        listing: c.listing,
        price_per_kwh: c.price_per_kwh,
        available_wh: c.wh - c.sold_wh,
    }
}

fn site_json(s: &Site, slug: &str) -> Value {
    let utc_offset = demo::by_slug(slug).map_or((s.lon / 15.0).round() as i32, |d| d.utc_offset);
    json!({ "slug": slug, "utc_offset": utc_offset, "name": s.name, "country": s.country, "lat": s.lat, "lon": s.lon, "device": s.device,
            "capacity_w": s.capacity_w, "has_generator": s.has_generator, "grid_factor": demo::grid_factor(&s.country) })
}

async fn network(State(app): A) -> Json<Value> {
    let t = now();
    let day = t - t % 86_400;
    let store = app.store.lock().unwrap();
    let sites: Vec<Value> = demo::SITES
        .iter()
        .map(|d| {
            let s = d.site();
            let today = store.readings_since(s.device, day);
            let last = today.last().filter(|r| r.ts + 900 > t);
            let mut v = site_json(&s, d.slug);
            v["live_w"] = json!(last.map(|r| r.wh * 12).unwrap_or(0));
            v["grid_down"] = json!(last.map(|r| r.gridV < GRID_DOWN_V).unwrap_or(false));
            v["sun"] = json!(solar::elevation(t as i64, s.lat, s.lon).round());
            v["today_wh"] = json!(today.iter().map(|r| r.wh as u64).sum::<u64>());
            v
        })
        .collect();
    let (wh, g, n, g_ret) = store.totals();
    let feed: Vec<_> = store.certs_recent(12).iter().map(|c| view(&app, c)).collect();
    let rejections = store.rejections(None, t - 6 * 3600, 8);
    Json(json!({
        "now": t,
        "totals": { "wh": wh, "g_co2": g, "certs": n, "g_retired": g_ret,
                    "live_w": sites.iter().map(|s| s["live_w"].as_u64().unwrap_or(0)).sum::<u64>() },
        "sites": sites, "feed": feed, "rejections": rejections,
    }))
}

async fn site(State(app): A, Path(slug): Path<String>) -> Response {
    let Some(d) = demo::by_slug(&slug) else { return not_found("site") };
    let s = d.site();
    let t = now();
    let store = app.store.lock().unwrap();
    let since_h = (t / 3600) as u32 - 23;
    let readings = store.readings_since(s.device, since_h as u64 * 3600);
    let certs = store.certs_of(s.device, since_h);
    let hours: Vec<Value> = (since_h..=(t / 3600) as u32)
        .map(|h| {
            let rs: Vec<_> = readings.iter().filter(|r| (r.ts - 1) / 3600 == h as u64).collect();
            let cert = certs.iter().find(|c| c.hour == h).map(|c| view(&app, c));
            json!({ "hour": h, "wh": rs.iter().map(|r| r.wh).sum::<u32>(), "readings": rs.len(),
                    "grid_down": !rs.is_empty() && rs.iter().all(|r| r.gridV < GRID_DOWN_V), "cert": cert })
        })
        .collect();
    let stream: Vec<Value> = readings
        .iter()
        .rev()
        .take(12)
        .map(|r| json!({ "ts": r.ts, "wh": r.wh, "grid_v": r.gridV, "nonce": r.nonce, "ok": true }))
        .collect();
    let rejections = store.rejections(Some(s.device), t - 3 * 3600, 6);
    let views: Vec<_> = certs.iter().map(|c| view(&app, c)).collect();
    let gen_g: u64 = certs.iter().filter(|c| c.source == "GEN").map(|c| c.g_co2).sum();
    Json(json!({
        "site": site_json(&s, d.slug),
        "now": t,
        "summary": { "wh": certs.iter().map(|c| c.wh as u64).sum::<u64>(), "g_co2": certs.iter().map(|c| c.g_co2).sum::<u64>(),
                     "g_co2_gen": gen_g, "certs": certs.len(), "gen_hours": certs.iter().filter(|c| c.source == "GEN").count(),
                     "earned": certs.iter().map(|c| c.sold_wh as u64 * c.price_per_kwh / 1000).sum::<u64>() },
        "hours": hours, "stream": stream, "rejections": rejections, "certs": views,
    }))
    .into_response()
}

async fn cert(State(app): A, Path(id): Path<String>) -> Response {
    let store = app.store.lock().unwrap();
    let id = id.to_lowercase();
    let c = store
        .cert(&id)
        .or_else(|| store.certs_recent(100_000).into_iter().find(|c| c.id.ends_with(&id.trim_start_matches('#')) || c.tx == id));
    let Some(c) = c else { return not_found("certificate") };
    let device: Address = c.device.parse().unwrap_or_default();
    let rs = store.hour_readings(device, c.hour);
    let leaves: Vec<_> = rs.iter().map(|(r, _)| leaf(r)).collect();
    let root = merkle::root(&leaves);
    let readings: Vec<Value> = rs
        .iter()
        .zip(&leaves)
        .map(|((r, sig), l)| json!({ "ts": r.ts, "wh": r.wh, "grid_v": r.gridV, "nonce": r.nonce, "sig": sig, "leaf": l }))
        .collect();
    let site = app.sites.get(&device);
    let retirements = store.retired_of(&c.id);
    Json(json!({
        "cert": view(&app, &c),
        "site": site.map(|s| site_json(s, &slug_of(&app, &c.device).0)),
        "readings": readings,
        "root": c.root,
        "root_matches": root.to_string() == c.root,
        "claim_sig": c.claim_sig,
        "attest_sig": c.attest_sig,
        "verifier": app.verifier.address(),
        "retirements": retirements,
    }))
    .into_response()
}

async fn market(State(app): A) -> Json<Value> {
    let store = app.store.lock().unwrap();
    let rows: Vec<_> = store.certs_for_sale(80).iter().map(|c| view(&app, c)).collect();
    Json(json!({ "certs": rows }))
}

#[derive(Deserialize)]
struct MatchReq {
    /// Hourly load for one day, kWh, index 0 = 00:00 UTC.
    load_kwh: Vec<f64>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Pick {
    id: String,
    listing: Option<u64>,
    wh: u32,
}

/// Greedy hourly matching against certificates from the last 24 hours, cheapest per tonne first.
async fn match_load(State(app): A, Json(req): Json<MatchReq>) -> Json<Value> {
    let store = app.store.lock().unwrap();
    let t = now();
    let mut pool: Vec<CertRow> = store.certs_for_sale(1000).into_iter().filter(|c| c.hour as u64 * 3600 + 86_400 > t).collect();
    pool.sort_by(|a, b| {
        let pa = a.price_per_kwh as f64 / a.factor.max(1) as f64;
        let pb = b.price_per_kwh as f64 / b.factor.max(1) as f64;
        pa.total_cmp(&pb)
    });
    let mut left: Vec<u32> = pool.iter().map(|c| c.wh - c.sold_wh).collect();
    let mut hours = Vec::with_capacity(24);
    let mut picks: Vec<Pick> = Vec::new();
    let (mut demand_t, mut matched_t, mut g_t, mut cost_t) = (0f64, 0f64, 0u64, 0u64);
    for h in 0..24u32 {
        let demand = (req.load_kwh.get(h as usize).copied().unwrap_or(0.0) * 1000.0) as u32;
        let (mut need, mut gen_wh) = (demand, 0u32);
        for (i, c) in pool.iter().enumerate() {
            if need == 0 { break; }
            if c.hour % 24 != h || left[i] == 0 { continue; }
            let take = need.min(left[i]);
            left[i] -= take;
            need -= take;
            if c.source == "GEN" { gen_wh += take; }
            g_t += c.g_co2 * take as u64 / c.wh as u64;
            cost_t += (take as u64 * c.price_per_kwh).div_ceil(1000);
            match picks.iter_mut().find(|p| p.id == c.id) {
                Some(p) => p.wh += take,
                None => picks.push(Pick { id: c.id.clone(), listing: c.listing, wh: take }),
            }
        }
        let matched = demand - need;
        demand_t += demand as f64;
        matched_t += matched as f64;
        hours.push(json!({ "hour": h, "demand_wh": demand, "matched_wh": matched, "gen_wh": gen_wh }));
    }
    let pct = if demand_t > 0.0 { (matched_t / demand_t * 100.0).round() } else { 0.0 };
    Json(json!({ "hours": hours, "match_pct": pct, "matched_wh": matched_t, "demand_wh": demand_t,
                 "g_co2": g_t, "cost": cost_t, "picks": picks, "certificates": picks.len() }))
}

#[derive(Deserialize)]
struct RetireReq {
    picks: Vec<Pick>,
    beneficiary: String,
}

async fn retire(State(app): A, Json(req): Json<RetireReq>) -> Response {
    let rows: Vec<(Pick, CertRow)> = {
        let store = app.store.lock().unwrap();
        req.picks.iter().filter_map(|p| Some((p.clone(), store.cert(&p.id)?))).collect()
    };
    if rows.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "nothing to retire" }))).into_response();
    }
    let g: u64 = rows.iter().map(|(p, c)| c.g_co2 * p.wh as u64 / c.wh as u64).sum();
    let cost: u64 = rows.iter().map(|(p, c)| (p.wh as u64 * c.price_per_kwh).div_ceil(1000)).sum();
    let listings: Vec<u64> = rows.iter().filter_map(|(_, c)| c.listing).collect();
    if app.chain.is_live() && listings.len() != rows.len() {
        return (StatusCode::CONFLICT, Json(json!({ "error": "some certificates are not listed on-chain" }))).into_response();
    }
    let whs: Vec<u32> = rows.iter().map(|(p, _)| p.wh).collect();
    let beneficiary = if req.beneficiary.trim().is_empty() { "[COMPANY]" } else { req.beneficiary.trim() };
    match app.chain.buy_and_retire(&listings, &whs, beneficiary, g, cost).await {
        Err(e) => (StatusCode::BAD_GATEWAY, Json(json!({ "error": e }))).into_response(),
        Ok(r) => {
            let row = RetirementRow {
                receipt: r.receipt,
                beneficiary: beneficiary.into(),
                cert_ids: rows.iter().map(|(_, c)| c.id.clone()).collect(),
                whs,
                g_co2: r.g_co2,
                cost: r.cost,
                tx: r.tx.map(|t| t.hash.to_string()).unwrap_or_default(),
                at: now(),
            };
            let store = app.store.lock().unwrap();
            for (p, c) in &rows {
                store.mark_sold(&c.id, p.wh);
            }
            let _ = store.insert_retirement(&row);
            Json(json!(row)).into_response()
        }
    }
}

async fn receipt(State(app): A, Path(n): Path<u64>) -> Response {
    let store = app.store.lock().unwrap();
    let Some(r) = store.retirements(Some(n), 1).pop() else { return not_found("receipt") };
    let certs: Vec<_> = r.cert_ids.iter().filter_map(|id| store.cert(id)).map(|c| view(&app, &c)).collect();
    Json(json!({ "receipt": r, "certs": certs })).into_response()
}

async fn attack(State(app): A, Path(kind): Path<String>) -> Response {
    match attacks::run(&app, &kind).await {
        Some(o) => Json(json!(o)).into_response(),
        None => (StatusCode::CONFLICT, Json(json!({ "error": "not enough live data for this attack yet; wait for the simulator to post an hour" }))).into_response(),
    }
}
