//! Sorted-pair keccak Merkle tree (OpenZeppelin MerkleProof compatible).

use alloy::primitives::{B256, keccak256};

fn pair(a: B256, b: B256) -> B256 {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(lo.as_slice());
    buf[32..].copy_from_slice(hi.as_slice());
    keccak256(buf)
}

pub fn root(leaves: &[B256]) -> B256 {
    if leaves.is_empty() {
        return B256::ZERO;
    }
    let mut level = leaves.to_vec();
    while level.len() > 1 {
        level = level
            .chunks(2)
            .map(|c| if let [a, b] = c { pair(*a, *b) } else { c[0] })
            .collect();
    }
    level[0]
}

pub fn proof(leaves: &[B256], mut i: usize) -> Vec<B256> {
    let mut out = Vec::new();
    let mut level = leaves.to_vec();
    while level.len() > 1 {
        let sib = i ^ 1;
        if sib < level.len() {
            out.push(level[sib]);
        }
        level = level
            .chunks(2)
            .map(|c| if let [a, b] = c { pair(*a, *b) } else { c[0] })
            .collect();
        i /= 2;
    }
    out
}

pub fn verify(leaf: B256, proof: &[B256], root: B256) -> bool {
    proof.iter().fold(leaf, |acc, p| pair(acc, *p)) == root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proofs_verify() {
        let leaves: Vec<B256> = (0u8..12).map(|i| keccak256([i])).collect();
        let r = root(&leaves);
        for (i, l) in leaves.iter().enumerate() {
            assert!(verify(*l, &proof(&leaves, i), r), "leaf {i}");
        }
        assert!(!verify(keccak256([99u8]), &proof(&leaves, 0), r));
    }
}
