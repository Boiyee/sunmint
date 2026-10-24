//! The demo network: six sites in six countries. Keys are derived from public seeds, so the simulator,
//! the verifier's Break it rig and the registration script all agree. Demo only, never for real meters.

use crate::checks::Site;
use alloy::{primitives::keccak256, signers::local::PrivateKeySigner};

pub struct DemoSite {
    pub slug: &'static str,
    pub name: &'static str,
    pub country: &'static str,
    pub lat: f64,
    pub lon: f64,
    pub capacity_w: u32,
    pub has_generator: bool,
    /// Local hours (site time) when the grid is down on a typical day.
    pub outages: &'static [u32],
    /// UTC offset in hours, for outage schedules.
    pub utc_offset: i32,
}

pub const SITES: [DemoSite; 6] = [
    DemoSite { slug: "lagos", name: "Lagos, NG", country: "NG", lat: 6.52, lon: 3.38, capacity_w: 7200, has_generator: true, outages: &[7, 10, 11, 19, 20, 21], utc_offset: 1 },
    DemoSite { slug: "nairobi", name: "Nairobi, KE", country: "KE", lat: -1.29, lon: 36.82, capacity_w: 8000, has_generator: true, outages: &[7, 8, 18], utc_offset: 3 },
    DemoSite { slug: "jaipur", name: "Jaipur, IN", country: "IN", lat: 26.91, lon: 75.79, capacity_w: 22000, has_generator: false, outages: &[], utc_offset: 5 },
    DemoSite { slug: "manila", name: "Manila, PH", country: "PH", lat: 14.6, lon: 120.98, capacity_w: 6000, has_generator: true, outages: &[14, 15], utc_offset: 8 },
    DemoSite { slug: "seville", name: "Seville, ES", country: "ES", lat: 37.39, lon: -5.98, capacity_w: 11000, has_generator: false, outages: &[], utc_offset: 1 },
    DemoSite { slug: "saopaulo", name: "São Paulo, BR", country: "BR", lat: -23.55, lon: -46.63, capacity_w: 5000, has_generator: false, outages: &[], utc_offset: -3 },
];

impl DemoSite {
    pub fn key(&self) -> PrivateKeySigner {
        PrivateKeySigner::from_bytes(&keccak256(format!("sunmint-demo-meter-{}", self.slug))).expect("valid scalar")
    }

    pub fn site(&self) -> Site {
        Site {
            device: self.key().address(),
            name: self.name.into(),
            country: self.country.into(),
            lat: self.lat,
            lon: self.lon,
            capacity_w: self.capacity_w,
            has_generator: self.has_generator,
        }
    }

    pub fn grid_down_at(&self, ts: u64) -> bool {
        let local = ((ts as i64 + self.utc_offset as i64 * 3600).rem_euclid(86_400) / 3600) as u32;
        self.outages.contains(&local)
    }
}

pub fn by_slug(slug: &str) -> Option<&'static DemoSite> {
    SITES.iter().find(|s| s.slug == slug)
}

/// Demo emission factors, g CO2 per kWh. Must match what Deploy.s.sol sets on-chain.
pub const GEN_FACTOR: u32 = 860;

pub fn grid_factor(country: &str) -> u32 {
    match country {
        "NG" => 400,
        "IN" => 710,
        "PH" => 650,
        "ES" => 150,
        "KE" => 100,
        "BR" => 90,
        _ => 500,
    }
}
