//! Chain access. `Live` talks to Base Sepolia; `Dry` mirrors the contract rules in memory so the
//! whole demo runs without keys or RPC.

use alloy::{
    network::EthereumWallet,
    primitives::{Address, B256, Bytes, U256, keccak256},
    providers::{DynProvider, Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
    sol,
    sol_types::SolValue,
};
use anyhow::{Result, anyhow};
use std::{collections::HashSet, sync::Mutex};
use sunmint_core::HourClaim;

sol! {
    #[sol(rpc)]
    contract SunmintCertificates {
        struct HourClaim { address device; uint32 hour; uint32 wh; bool gridDown; bytes32 readingsRoot; }
        function mint(HourClaim calldata c, bytes calldata deviceSig, bytes calldata verifierSig) external returns (uint256 id);
        function registerDevice(address device, uint32 capacityW, bool hasGenerator, bytes2 country, bytes calldata deviceSig) external;
        function devices(address) external view returns (address owner, uint32 capacityW, bool hasGenerator, bytes2 country);
        function setApprovalForAll(address operator, bool approved) external;
        function isApprovedForAll(address account, address operator) external view returns (bool);
        error UnknownDevice(address device);
        error DeviceExists(address device);
        error BadDeviceSignature();
        error BadVerifierSignature();
        error HourNotOver(uint32 hour);
        error AboveCapacity(uint32 wh, uint256 maxWh);
        error NoGridFactor(bytes2 country);
        error DeviceHourAlreadyMinted(address device, uint32 hour);
        event Retired(uint256 indexed receipt, address indexed by, string beneficiary, uint256[] ids, uint256[] whs, uint256 gCO2);
    }

    #[sol(rpc)]
    contract SunmintMarket {
        function list(uint256 id, uint256 wh, uint256 pricePerKwh) external returns (uint256 listing);
        function listingCount() external view returns (uint256);
        function buyAndRetire(uint256[] calldata ls, uint256[] calldata whs, string calldata beneficiary) external returns (uint256 receipt, uint256 gCO2, uint256 cost);
        event Listed(uint256 indexed listing, address indexed seller, uint256 indexed id, uint256 wh, uint256 pricePerKwh);
        event RetiredFor(uint256 indexed receipt, address indexed buyer, uint256 cost);
        error NotEnough(uint256 available);
    }

    #[sol(rpc)]
    contract TestUSD {
        function faucet(uint256 amount) external;
        function approve(address spender, uint256 amount) external returns (bool);
        function balanceOf(address) external view returns (uint256);
    }
}

pub fn certificate_id(device: Address, hour: u32) -> U256 {
    U256::from_be_bytes(keccak256((device, hour).abi_encode()).0)
}

fn to_sol(c: &HourClaim) -> SunmintCertificates::HourClaim {
    SunmintCertificates::HourClaim { device: c.device, hour: c.hour, wh: c.wh, gridDown: c.gridDown, readingsRoot: c.readingsRoot }
}

#[derive(Debug, Clone)]
pub struct Tx {
    pub hash: B256,
    pub block: u64,
}

#[derive(Debug, Clone, Default)]
pub struct Retirement {
    pub receipt: u64,
    pub g_co2: u64,
    pub cost: u64,
    pub tx: Option<Tx>,
}

pub struct Live {
    producer: DynProvider,
    producer_addr: Address,
    company: DynProvider,
    company_addr: Address,
    certs: Address,
    market: Address,
    usd: Address,
}

pub enum Chain {
    Live(Live),
    Dry { minted: Mutex<HashSet<(Address, u32)>>, seq: Mutex<u64>, receipts: Mutex<u64> },
}

/// Turns a revert into the contract's error name and arguments.
fn revert_reason(e: &alloy::contract::Error) -> String {
    use SunmintCertificates::SunmintCertificatesErrors as E;
    match e.as_decoded_interface_error::<E>() {
        Some(E::DeviceHourAlreadyMinted(x)) => format!("revert DeviceHourAlreadyMinted({}, hour {})", sunmint_core::checks::short(x.device), x.hour),
        Some(E::BadDeviceSignature(_)) => "revert BadDeviceSignature()".into(),
        Some(E::BadVerifierSignature(_)) => "revert BadVerifierSignature()".into(),
        Some(E::HourNotOver(x)) => format!("revert HourNotOver({})", x.hour),
        Some(E::AboveCapacity(x)) => format!("revert AboveCapacity({}, {})", x.wh, x.maxWh),
        Some(E::UnknownDevice(x)) => format!("revert UnknownDevice({})", x.device),
        Some(_) => format!("revert {e}"),
        None => e.to_string(),
    }
}

impl Chain {
    pub fn dry() -> Self {
        Chain::Dry { minted: Mutex::default(), seq: Mutex::new(0), receipts: Mutex::new(0) }
    }

    pub fn live(rpc: &str, producer: PrivateKeySigner, company: PrivateKeySigner, certs: Address, market: Address, usd: Address) -> Result<Self> {
        let url: alloy::transports::http::reqwest::Url = rpc.parse()?;
        let company_addr = company.address();
        let producer_addr = producer.address();
        let p = ProviderBuilder::new().wallet(EthereumWallet::from(producer)).connect_http(url.clone()).erased();
        let c = ProviderBuilder::new().wallet(EthereumWallet::from(company)).connect_http(url).erased();
        Ok(Chain::Live(Live { producer: p, producer_addr, company: c, company_addr, certs, market, usd }))
    }

    pub fn is_live(&self) -> bool {
        matches!(self, Chain::Live(_))
    }

    fn dry_tx(seq: &Mutex<u64>) -> Tx {
        let mut n = seq.lock().unwrap();
        *n += 1;
        Tx { hash: keccak256(format!("dry-{n}")), block: 18_200_000 + *n }
    }

    /// Simulates first so a revert comes back as a readable reason, then sends.
    pub async fn mint(&self, c: &HourClaim, dev_sig: Bytes, att_sig: Bytes) -> Result<Tx, String> {
        match self {
            Chain::Dry { minted, seq, .. } => {
                if !minted.lock().unwrap().insert((c.device, c.hour)) {
                    return Err(format!("revert DeviceHourAlreadyMinted({}, hour {})", sunmint_core::checks::short(c.device), c.hour));
                }
                Ok(Self::dry_tx(seq))
            }
            Chain::Live(l) => {
                let k = SunmintCertificates::new(l.certs, &l.producer);
                let call = k.mint(to_sol(c), dev_sig, att_sig);
                call.call().await.map_err(|e| revert_reason(&e))?;
                let r = call.send().await.map_err(|e| revert_reason(&e))?.get_receipt().await.map_err(|e| e.to_string())?;
                if !r.status() {
                    return Err("mint transaction reverted".into());
                }
                Ok(Tx { hash: r.transaction_hash, block: r.block_number.unwrap_or_default() })
            }
        }
    }

    /// Lists a freshly minted certificate on the market from the producer wallet.
    pub async fn list(&self, id: U256, wh: u32, price_per_kwh: u64) -> Result<Option<u64>> {
        let Chain::Live(l) = self else { return Ok(None) };
        let m = SunmintMarket::new(l.market, &l.producer);
        let r = m.list(id, U256::from(wh), U256::from(price_per_kwh)).send().await?.get_receipt().await?;
        let listing = r
            .inner
            .logs()
            .iter()
            .find_map(|log| log.log_decode::<SunmintMarket::Listed>().ok())
            .map(|l| l.inner.listing.to::<u64>());
        Ok(listing)
    }

    pub async fn buy_and_retire(&self, listings: &[u64], whs: &[u32], beneficiary: &str, g_co2_dry: u64, cost_dry: u64) -> Result<Retirement, String> {
        match self {
            Chain::Dry { seq, receipts, .. } => {
                let tx = Self::dry_tx(seq);
                let mut n = receipts.lock().unwrap();
                *n += 1;
                Ok(Retirement { receipt: *n, g_co2: g_co2_dry, cost: cost_dry, tx: Some(tx) })
            }
            Chain::Live(l) => {
                let m = SunmintMarket::new(l.market, &l.company);
                let ls = listings.iter().map(|&x| U256::from(x)).collect::<Vec<_>>();
                let ws = whs.iter().map(|&x| U256::from(x)).collect::<Vec<_>>();
                let call = m.buyAndRetire(ls, ws, beneficiary.to_owned());
                let out = call.call().await.map_err(|e| e.to_string())?;
                let r = call.send().await.map_err(|e| e.to_string())?.get_receipt().await.map_err(|e| e.to_string())?;
                Ok(Retirement {
                    receipt: out.receipt.to(),
                    g_co2: out.gCO2.to(),
                    cost: out.cost.to(),
                    tx: Some(Tx { hash: r.transaction_hash, block: r.block_number.unwrap_or_default() }),
                })
            }
        }
    }

    /// One-off: register the demo meters to the producer wallet, approve the market, fund the company.
    pub async fn setup(&self, regs: Vec<(Address, u32, bool, [u8; 2], Bytes)>) -> Result<()> {
        let Chain::Live(l) = self else { return Err(anyhow!("setup needs RPC_URL and contract addresses")) };
        let k = SunmintCertificates::new(l.certs, &l.producer);
        for (device, cap, generator, cc, sig) in regs {
            if k.devices(device).call().await?.owner != Address::ZERO {
                println!("  {device} already registered");
                continue;
            }
            k.registerDevice(device, cap, generator, cc.into(), sig).send().await?.get_receipt().await?;
            println!("  registered {device}");
        }
        let producer = l.producer_addr;
        if !k.isApprovedForAll(producer, l.market).call().await? {
            k.setApprovalForAll(l.market, true).send().await?.get_receipt().await?;
            println!("  market approved for producer");
        }
        let usd = TestUSD::new(l.usd, &l.company);
        if usd.balanceOf(l.company_addr).call().await? < U256::from(1_000_000_000u64) {
            usd.faucet(U256::from(10_000_000_000u64)).send().await?.get_receipt().await?;
            usd.approve(l.market, U256::MAX).send().await?.get_receipt().await?;
            println!("  company funded with 10,000 tUSDC and approved the market");
        }
        Ok(())
    }

    pub fn addresses(&self) -> Option<(Address, Address, Address)> {
        match self {
            Chain::Live(l) => Some((l.certs, l.market, l.usd)),
            Chain::Dry { .. } => None,
        }
    }
}
