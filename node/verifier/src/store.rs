//! SQLite store for accepted readings, rejections, certificates and retirements.

use alloy::primitives::{Address, Signature};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use sunmint_core::{Reading, checks::DeviceState};

pub struct Store(Connection);

#[derive(Debug, Clone, Serialize)]
pub struct CertRow {
    pub id: String,
    pub device: String,
    pub hour: u32,
    pub wh: u32,
    pub source: String,
    pub factor: u32,
    pub g_co2: u64,
    pub root: String,
    pub claim_sig: String,
    pub attest_sig: String,
    pub tx: String,
    pub block: u64,
    pub minted_at: u64,
    pub listing: Option<u64>,
    pub price_per_kwh: u64,
    pub sold_wh: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Rejection {
    pub device: String,
    pub kind: String,
    pub ts: u64,
    pub verdict: String,
    pub reason: String,
    pub at: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetirementRow {
    pub receipt: u64,
    pub beneficiary: String,
    pub cert_ids: Vec<String>,
    pub whs: Vec<u32>,
    pub g_co2: u64,
    pub cost: u64,
    pub tx: String,
    pub at: u64,
}

impl Store {
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        let c = Connection::open(path)?;
        c.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS readings(device TEXT, ts INT, hour INT, wh INT, grid_v INT, nonce INT, sig TEXT, PRIMARY KEY(device, nonce));
             CREATE INDEX IF NOT EXISTS readings_hour ON readings(device, hour);
             CREATE TABLE IF NOT EXISTS state(device TEXT PRIMARY KEY, last_nonce INT, last_ts INT);
             CREATE TABLE IF NOT EXISTS rejections(id INTEGER PRIMARY KEY, device TEXT, kind TEXT, ts INT, verdict TEXT, reason TEXT, at INT);
             CREATE TABLE IF NOT EXISTS certs(id TEXT PRIMARY KEY, device TEXT, hour INT, wh INT, source TEXT, factor INT, g_co2 INT, root TEXT,
                claim_sig TEXT, attest_sig TEXT, tx TEXT, block INT, minted_at INT, listing INT, price_per_kwh INT, sold_wh INT DEFAULT 0);
             CREATE INDEX IF NOT EXISTS certs_device ON certs(device, hour);
             CREATE TABLE IF NOT EXISTS retirements(receipt INT PRIMARY KEY, beneficiary TEXT, cert_ids TEXT, whs TEXT, g_co2 INT, cost INT, tx TEXT, at INT);",
        )?;
        Ok(Self(c))
    }

    pub fn state(&self, device: Address) -> DeviceState {
        self.0
            .query_row("SELECT last_nonce, last_ts FROM state WHERE device=?1", [device.to_string()], |r| {
                Ok(DeviceState { last_nonce: r.get(0)?, last_ts: r.get(1)? })
            })
            .optional()
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    pub fn accept(&self, r: &Reading, sig: &Signature) -> rusqlite::Result<()> {
        let d = r.device.to_string();
        self.0.execute(
            "INSERT OR IGNORE INTO readings VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![d, r.ts, (r.ts - 1) / 3600, r.wh, r.gridV, r.nonce, sig.to_string()],
        )?;
        self.0.execute(
            "INSERT INTO state VALUES(?1,?2,?3) ON CONFLICT(device) DO UPDATE SET last_nonce=?2, last_ts=?3",
            params![d, r.nonce, r.ts],
        )?;
        Ok(())
    }

    pub fn reject(&self, device: Address, kind: &str, ts: u64, verdict: &str, reason: &str, at: u64) {
        let _ = self.0.execute(
            "INSERT INTO rejections(device, kind, ts, verdict, reason, at) VALUES(?1,?2,?3,?4,?5,?6)",
            params![device.to_string(), kind, ts, verdict, reason, at],
        );
    }

    /// Readings of one device-hour, ordered. An interval ending exactly on the hour belongs to that hour.
    pub fn hour_readings(&self, device: Address, hour: u32) -> Vec<(Reading, String)> {
        let mut st = self.0.prepare_cached("SELECT ts, wh, grid_v, nonce, sig FROM readings WHERE device=?1 AND hour=?2 ORDER BY ts").unwrap();
        st.query_map(params![device.to_string(), hour], |r| {
            Ok((Reading { device, ts: r.get(0)?, wh: r.get(1)?, gridV: r.get(2)?, nonce: r.get(3)? }, r.get::<_, String>(4)?))
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect()
    }

    pub fn readings_since(&self, device: Address, since: u64) -> Vec<Reading> {
        let mut st = self.0.prepare_cached("SELECT ts, wh, grid_v, nonce FROM readings WHERE device=?1 AND ts>?2 ORDER BY ts").unwrap();
        st.query_map(params![device.to_string(), since], |r| {
            Ok(Reading { device, ts: r.get(0)?, wh: r.get(1)?, gridV: r.get(2)?, nonce: r.get(3)? })
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect()
    }

    pub fn last_reading_sig(&self, device: Address) -> Option<(Reading, String)> {
        self.0
            .query_row("SELECT ts, wh, grid_v, nonce, sig FROM readings WHERE device=?1 ORDER BY ts DESC LIMIT 1", [device.to_string()], |r| {
                Ok((Reading { device, ts: r.get(0)?, wh: r.get(1)?, gridV: r.get(2)?, nonce: r.get(3)? }, r.get::<_, String>(4)?))
            })
            .optional()
            .ok()
            .flatten()
    }

    pub fn rejections(&self, device: Option<Address>, since: u64, limit: u32) -> Vec<Rejection> {
        let dev = device.map(|d| d.to_string()).unwrap_or_default();
        let mut st = self
            .0
            .prepare_cached("SELECT device, kind, ts, verdict, reason, at FROM rejections WHERE (?1='' OR device=?1) AND at>?2 ORDER BY id DESC LIMIT ?3")
            .unwrap();
        st.query_map(params![dev, since, limit], |r| {
            Ok(Rejection { device: r.get(0)?, kind: r.get(1)?, ts: r.get(2)?, verdict: r.get(3)?, reason: r.get(4)?, at: r.get(5)? })
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect()
    }

    pub fn insert_cert(&self, c: &CertRow) -> rusqlite::Result<()> {
        self.0.execute(
            "INSERT OR REPLACE INTO certs VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![c.id, c.device, c.hour, c.wh, c.source, c.factor, c.g_co2, c.root, c.claim_sig, c.attest_sig, c.tx, c.block, c.minted_at, c.listing, c.price_per_kwh, c.sold_wh],
        )?;
        Ok(())
    }

    pub fn has_cert(&self, device: Address, hour: u32) -> bool {
        self.0
            .query_row("SELECT 1 FROM certs WHERE device=?1 AND hour=?2", params![device.to_string(), hour], |_| Ok(()))
            .optional()
            .ok()
            .flatten()
            .is_some()
    }

    fn certs_where(&self, clause: &str, p: impl rusqlite::Params) -> Vec<CertRow> {
        let sql = format!("SELECT * FROM certs {clause}");
        let mut st = self.0.prepare(&sql).unwrap();
        st.query_map(p, |r| {
            Ok(CertRow {
                id: r.get(0)?,
                device: r.get(1)?,
                hour: r.get(2)?,
                wh: r.get(3)?,
                source: r.get(4)?,
                factor: r.get(5)?,
                g_co2: r.get(6)?,
                root: r.get(7)?,
                claim_sig: r.get(8)?,
                attest_sig: r.get(9)?,
                tx: r.get(10)?,
                block: r.get(11)?,
                minted_at: r.get(12)?,
                listing: r.get(13)?,
                price_per_kwh: r.get(14)?,
                sold_wh: r.get(15)?,
            })
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect()
    }

    pub fn cert(&self, id: &str) -> Option<CertRow> {
        self.certs_where("WHERE id=?1", [id]).pop()
    }

    pub fn certs_recent(&self, limit: u32) -> Vec<CertRow> {
        self.certs_where("ORDER BY minted_at DESC, hour DESC LIMIT ?1", [limit])
    }

    pub fn certs_of(&self, device: Address, since_hour: u32) -> Vec<CertRow> {
        self.certs_where("WHERE device=?1 AND hour>=?2 ORDER BY hour", params![device.to_string(), since_hour])
    }

    pub fn certs_for_sale(&self, limit: u32) -> Vec<CertRow> {
        self.certs_where("WHERE sold_wh < wh ORDER BY hour DESC LIMIT ?1", [limit])
    }

    pub fn mark_sold(&self, id: &str, wh: u32) {
        let _ = self.0.execute("UPDATE certs SET sold_wh = sold_wh + ?2 WHERE id=?1", params![id, wh]);
    }

    pub fn insert_retirement(&self, r: &RetirementRow) -> rusqlite::Result<()> {
        self.0.execute(
            "INSERT OR REPLACE INTO retirements VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![r.receipt, r.beneficiary, serde_json::to_string(&r.cert_ids).unwrap(), serde_json::to_string(&r.whs).unwrap(), r.g_co2, r.cost, r.tx, r.at],
        )?;
        Ok(())
    }

    pub fn retirements(&self, receipt: Option<u64>, limit: u32) -> Vec<RetirementRow> {
        let mut st = self
            .0
            .prepare_cached("SELECT * FROM retirements WHERE (?1 IS NULL OR receipt=?1) ORDER BY receipt DESC LIMIT ?2")
            .unwrap();
        st.query_map(params![receipt, limit], |r| {
            Ok(RetirementRow {
                receipt: r.get(0)?,
                beneficiary: r.get(1)?,
                cert_ids: serde_json::from_str(&r.get::<_, String>(2)?).unwrap_or_default(),
                whs: serde_json::from_str(&r.get::<_, String>(3)?).unwrap_or_default(),
                g_co2: r.get(4)?,
                cost: r.get(5)?,
                tx: r.get(6)?,
                at: r.get(7)?,
            })
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect()
    }

    pub fn retired_of(&self, cert: &str) -> Vec<RetirementRow> {
        self.retirements(None, 500).into_iter().filter(|r| r.cert_ids.iter().any(|c| c == cert)).collect()
    }

    pub fn totals(&self) -> (u64, u64, u64, u64) {
        self.0
            .query_row(
                "SELECT COALESCE(SUM(wh),0), COALESCE(SUM(g_co2),0), COUNT(*), (SELECT COALESCE(SUM(g_co2),0) FROM retirements) FROM certs",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap_or_default()
    }
}
