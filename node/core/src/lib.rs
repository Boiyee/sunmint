//! Shared SUNMINT types: the signed messages, Merkle commitments, solar physics and the check pipeline.

pub mod checks;
pub mod demo;
pub mod merkle;
pub mod solar;

use alloy::{
    primitives::{Address, B256, Signature},
    signers::{SignerSync, local::PrivateKeySigner},
    sol,
    sol_types::{Eip712Domain, SolStruct, eip712_domain},
};
use serde::{Deserialize, Serialize};

sol! {
    /// One 5-minute reading, signed by the meter.
    #[derive(Debug, Copy, Serialize, Deserialize, PartialEq, Eq)]
    struct Reading {
        address device;
        uint64 ts;      // unix seconds, end of the interval
        uint32 wh;      // energy in the interval
        uint16 gridV;   // grid voltage seen at the changeover switch
        uint64 nonce;   // strictly increasing per device
    }

    /// The hour summary the meter signs and the contract verifies.
    #[derive(Debug, Copy, Serialize, Deserialize, PartialEq, Eq)]
    struct HourClaim {
        address device;
        uint32 hour;
        uint32 wh;
        bool gridDown;
        bytes32 readingsRoot;
    }

    /// The verifier's co-signature over a claim digest.
    struct Attest {
        bytes32 claimDigest;
    }
}

pub const CHAIN_ID: u64 = 84532; // Base Sepolia
pub const READINGS_PER_HOUR: usize = 12;

pub fn domain(contract: Address, chain_id: u64) -> Eip712Domain {
    eip712_domain! {
        name: "SUNMINT",
        version: "1",
        chain_id: chain_id,
        verifying_contract: contract,
    }
}

pub fn sign<T: SolStruct>(msg: &T, d: &Eip712Domain, key: &PrivateKeySigner) -> Signature {
    key.sign_hash_sync(&msg.eip712_signing_hash(d)).expect("k256 signing is infallible")
}

pub fn signer_of<T: SolStruct>(msg: &T, d: &Eip712Domain, sig: &Signature) -> Option<Address> {
    sig.recover_address_from_prehash(&msg.eip712_signing_hash(d)).ok()
}

pub fn attest(claim: &HourClaim, d: &Eip712Domain, verifier: &PrivateKeySigner) -> Signature {
    sign(&Attest { claimDigest: claim.eip712_signing_hash(d) }, d, verifier)
}

/// Leaf hash used in the hour's Merkle tree: the reading's EIP-712 struct hash.
pub fn leaf(r: &Reading) -> B256 {
    r.eip712_hash_struct()
}

/// Wire format: a reading plus its signature as 0x-hex.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedReading {
    pub reading: Reading,
    pub sig: Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedClaim {
    pub claim: HourClaim,
    pub sig: Signature,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_recover_roundtrip() {
        let key = PrivateKeySigner::random();
        let d = domain(Address::repeat_byte(7), CHAIN_ID);
        let r = Reading { device: key.address(), ts: 1_771_000_000, wh: 470, gridV: 229, nonce: 1 };
        let sig = sign(&r, &d, &key);
        assert_eq!(signer_of(&r, &d, &sig), Some(key.address()));
        let tampered = Reading { wh: 4700, ..r };
        assert_ne!(signer_of(&tampered, &d, &sig), Some(key.address()));
    }
}
