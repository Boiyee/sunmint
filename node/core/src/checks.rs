//! The verifier pipeline. Every reading and every hour claim runs through the same five stages,
//! and the stage list is what the Break it page shows.

use crate::{HourClaim, Reading, SignedClaim, SignedReading, leaf, merkle, signer_of, solar};
use alloy::{primitives::Address, sol_types::Eip712Domain};
use serde::{Deserialize, Serialize};

pub const INTERVAL_S: u64 = 300;
pub const TOLERANCE: f64 = 1.05;
/// Below this the grid counts as down.
pub const GRID_DOWN_V: u16 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StageName {
    Signature,
    Freshness,
    Physics,
    Displacement,
    OnChain,
}

pub const STAGES: [StageName; 5] =
    [StageName::Signature, StageName::Freshness, StageName::Physics, StageName::Displacement, StageName::OnChain];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pass,
    Reject,
    Downgrade,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub name: StageName,
    pub status: Status,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub stages: Vec<Stage>,
    pub verdict: Status,
    pub reason: String,
}

impl Report {
    /// Fills stages up to `n` as passed, stage `n` with `status`, the rest skipped (or passed on downgrade).
    fn stop(passed: Vec<Stage>, name: StageName, status: Status, reason: impl Into<String>) -> Self {
        let reason = reason.into();
        let mut stages = passed;
        stages.push(Stage { name, status, detail: reason.clone() });
        let rest = if status == Status::Downgrade { Status::Pass } else { Status::Skipped };
        stages.extend(STAGES.iter().skip(stages.len()).map(|&name| Stage { name, status: rest, detail: String::new() }));
        Self { stages, verdict: status, reason }
    }

    pub fn passed(&self) -> bool {
        self.verdict == Status::Pass
    }
}

fn pass(name: StageName, detail: impl Into<String>) -> Stage {
    Stage { name, status: Status::Pass, detail: detail.into() }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    pub device: Address,
    pub name: String,
    pub country: String,
    pub lat: f64,
    pub lon: f64,
    pub capacity_w: u32,
    pub has_generator: bool,
}

impl Site {
    pub fn max_interval_wh(&self) -> f64 {
        self.capacity_w as f64 * INTERVAL_S as f64 / 3600.0 * TOLERANCE
    }
}

/// What the verifier remembers per device between readings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceState {
    pub last_nonce: u64,
    pub last_ts: u64,
}

pub fn check_reading(
    sr: &SignedReading,
    site: Option<&Site>,
    st: &DeviceState,
    now: u64,
    d: &Eip712Domain,
) -> Report {
    let r: &Reading = &sr.reading;
    let mut ok = Vec::new();

    let Some(site) = site else {
        return Report::stop(ok, StageName::Signature, Status::Reject, format!("Device {} is not registered.", r.device));
    };
    if signer_of(r, d, &sr.sig) != Some(r.device) {
        return Report::stop(ok, StageName::Signature, Status::Reject, format!("Signature does not match device key {}.", short(r.device)));
    }
    ok.push(pass(StageName::Signature, format!("signed by {}", short(r.device))));

    if r.nonce <= st.last_nonce {
        return Report::stop(ok, StageName::Freshness, Status::Reject, format!("Nonce {} already used (last {}).", r.nonce, st.last_nonce));
    }
    if r.ts <= st.last_ts {
        return Report::stop(ok, StageName::Freshness, Status::Reject, format!("Reading is older than the last one accepted ({} s behind).", st.last_ts - r.ts));
    }
    if r.ts > now + 60 {
        return Report::stop(ok, StageName::Freshness, Status::Reject, "Reading is timestamped in the future.");
    }
    ok.push(pass(StageName::Freshness, format!("nonce {}", r.nonce)));

    let mid = r.ts as i64 - INTERVAL_S as i64 / 2;
    let elev = solar::elevation(mid, site.lat, site.lon);
    if elev < -0.5 && r.wh > 0 {
        return Report::stop(ok, StageName::Physics, Status::Reject, format!("Sun is {:.0}° below the horizon at {}. No solar output is possible.", -elev, site.name));
    }
    let max = site.max_interval_wh();
    if r.wh as f64 > max {
        return Report::stop(ok, StageName::Physics, Status::Reject, format!("{} Wh in 5 min is above {:.0} Wh, the most a {:.1} kWp array can produce.", r.wh, max, site.capacity_w as f64 / 1000.0));
    }
    ok.push(pass(StageName::Physics, format!("sun {elev:.0}°, {} Wh ≤ {max:.0}", r.wh)));

    let down = r.gridV < GRID_DOWN_V;
    ok.push(pass(StageName::Displacement, if down { format!("grid {} V: down", r.gridV) } else { format!("grid {} V: up", r.gridV) }));
    Report::stop(ok, StageName::OnChain, Status::Pass, "Reading accepted; it will be committed in the hour's Merkle root.")
        .tap_pass()
}

/// Aggregates an hour's accepted readings into the claim an honest meter would sign.
pub fn honest_claim(device: Address, hour: u32, readings: &[Reading]) -> HourClaim {
    let leaves: Vec<_> = readings.iter().map(leaf).collect();
    HourClaim {
        device,
        hour,
        wh: readings.iter().map(|r| r.wh).sum(),
        gridDown: !readings.is_empty() && readings.iter().all(|r| r.gridV < GRID_DOWN_V),
        readingsRoot: merkle::root(&leaves),
    }
}

/// Checks a signed hour claim against the readings the verifier accepted for that hour.
/// The ON_CHAIN stage is left for the caller, which simulates and sends the mint.
pub fn check_claim(sc: &SignedClaim, site: Option<&Site>, readings: &[Reading], now: u64, d: &Eip712Domain) -> Report {
    let c = &sc.claim;
    let mut ok = Vec::new();
    let Some(site) = site else {
        return Report::stop(ok, StageName::Signature, Status::Reject, format!("Device {} is not registered.", c.device));
    };
    if signer_of(c, d, &sc.sig) != Some(c.device) {
        return Report::stop(ok, StageName::Signature, Status::Reject, format!("Signature does not match device key {}.", short(c.device)));
    }
    ok.push(pass(StageName::Signature, format!("signed by {}", short(c.device))));

    if (c.hour as u64 + 1) * 3600 > now {
        return Report::stop(ok, StageName::Freshness, Status::Reject, "That hour has not finished yet.");
    }
    if readings.is_empty() {
        return Report::stop(ok, StageName::Freshness, Status::Reject, "No accepted readings for that hour.");
    }
    ok.push(pass(StageName::Freshness, format!("{} readings for hour {}", readings.len(), c.hour)));

    let honest = honest_claim(c.device, c.hour, readings);
    if c.readingsRoot != honest.readingsRoot || c.wh != honest.wh {
        return Report::stop(ok, StageName::Physics, Status::Reject, format!("Claim says {} Wh but the signed readings add up to {} Wh.", c.wh, honest.wh));
    }
    let max = site.capacity_w as f64 * TOLERANCE;
    if c.wh as f64 > max {
        return Report::stop(ok, StageName::Physics, Status::Reject, format!("{} Wh in one hour is above {max:.0} Wh for this array.", c.wh));
    }
    ok.push(pass(StageName::Physics, format!("{} Wh matches root {}", c.wh, short_hash(&c.readingsRoot.to_string()))));

    if c.gridDown && !honest.gridDown {
        let v = readings.iter().map(|r| r.gridV).max().unwrap_or(0);
        return Report::stop(ok, StageName::Displacement, Status::Downgrade, format!("Grid read {v} V during the hour, so no generator was displaced. Only the honest grid-factor claim is minted."));
    }
    let src = match (c.gridDown, site.has_generator) {
        (true, true) => "GEN: grid 0 V all hour, generator registered",
        (true, false) => "GRID: blackout but no generator registered",
        _ => "GRID: grid was up",
    };
    ok.push(pass(StageName::Displacement, src));
    Report::stop(ok, StageName::OnChain, Status::Pass, "Ready to co-sign and mint.").tap_pass()
}

impl Report {
    /// The last stage of a fully passing report is a real pass, not a stop.
    fn tap_pass(mut self) -> Self {
        self.verdict = Status::Pass;
        self
    }
}

pub fn short(a: Address) -> String {
    let s = a.to_string();
    format!("{}…{}", &s[..6], &s[s.len() - 4..])
}

pub fn short_hash(s: &str) -> String {
    format!("{}…{}", &s[..6], &s[s.len() - 4..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CHAIN_ID, domain, sign};
    use alloy::signers::local::PrivateKeySigner;

    const NOON: u64 = 1_782_043_200; // 2026-06-21 12:00 UTC, ~13:00 in Lagos

    fn setup() -> (PrivateKeySigner, Site, Eip712Domain) {
        let key = PrivateKeySigner::random();
        let site = Site { device: key.address(), name: "Lagos".into(), country: "NG".into(), lat: 6.5, lon: 3.4, capacity_w: 7200, has_generator: true };
        (key, site, domain(Address::repeat_byte(1), CHAIN_ID))
    }

    fn reading(key: &PrivateKeySigner, d: &Eip712Domain, ts: u64, wh: u32, v: u16, nonce: u64) -> SignedReading {
        let reading = Reading { device: key.address(), ts, wh, gridV: v, nonce };
        SignedReading { sig: sign(&reading, d, key), reading }
    }

    #[test]
    fn honest_reading_passes() {
        let (k, s, d) = setup();
        let rep = check_reading(&reading(&k, &d, NOON, 520, 230, 1), Some(&s), &DeviceState::default(), NOON, &d);
        assert!(rep.passed(), "{rep:?}");
    }

    #[test]
    fn tampered_reading_rejected_at_signature() {
        let (k, s, d) = setup();
        let mut sr = reading(&k, &d, NOON, 520, 230, 1);
        sr.reading.wh = 5200;
        let rep = check_reading(&sr, Some(&s), &DeviceState::default(), NOON, &d);
        assert_eq!((rep.verdict, rep.stages[0].status), (Status::Reject, Status::Reject));
    }

    #[test]
    fn replay_rejected_at_freshness() {
        let (k, s, d) = setup();
        let st = DeviceState { last_nonce: 5, last_ts: NOON };
        let rep = check_reading(&reading(&k, &d, NOON - 86_400, 520, 230, 3), Some(&s), &st, NOON, &d);
        assert_eq!(rep.stages[1].status, Status::Reject);
    }

    #[test]
    fn night_and_overcapacity_rejected_at_physics() {
        let (k, s, d) = setup();
        let night = NOON - 11 * 3600;
        let rep = check_reading(&reading(&k, &d, night, 400, 230, 1), Some(&s), &DeviceState::default(), NOON, &d);
        assert_eq!(rep.stages[2].status, Status::Reject);
        let rep = check_reading(&reading(&k, &d, NOON, 950, 230, 1), Some(&s), &DeviceState::default(), NOON, &d);
        assert_eq!(rep.stages[2].status, Status::Reject);
    }

    #[test]
    fn fake_blackout_downgraded() {
        let (k, s, d) = setup();
        let hour = (NOON / 3600 - 1) as u32;
        let rs: Vec<Reading> = (1..=12).map(|i| reading(&k, &d, hour as u64 * 3600 + i * 300, 500, 231, i).reading).collect();
        let mut claim = honest_claim(k.address(), hour, &rs);
        assert!(!claim.gridDown);
        claim.gridDown = true;
        let sc = SignedClaim { sig: sign(&claim, &d, &k), claim };
        let rep = check_claim(&sc, Some(&s), &rs, NOON, &d);
        assert_eq!(rep.verdict, Status::Downgrade);
        assert_eq!(rep.stages[3].status, Status::Downgrade);
    }

    #[test]
    fn honest_gen_hour_passes() {
        let (k, s, d) = setup();
        let hour = (NOON / 3600 - 1) as u32;
        let rs: Vec<Reading> = (1..=12).map(|i| reading(&k, &d, hour as u64 * 3600 + i * 300, 500, 0, i).reading).collect();
        let claim = honest_claim(k.address(), hour, &rs);
        assert!(claim.gridDown);
        let sc = SignedClaim { sig: sign(&claim, &d, &k), claim };
        assert!(check_claim(&sc, Some(&s), &rs, NOON, &d).passed());
    }
}
