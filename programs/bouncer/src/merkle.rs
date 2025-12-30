use anchor_lang::prelude::*;

use crate::{
    errors::BouncerError,
    state::{MERKLE_MAX_DEPTH, POLICY_ALLOWLIST, POLICY_BLOCKLIST},
};

const ZERO_LEAF: [u8; 32] = [0u8; 32];

fn hash_leaf_one(key_hash: &[u8; 32]) -> [u8; 32] {
    solana_sha256_hasher::hashv(&[
        b"bouncer:leaf".as_ref(),
        key_hash.as_ref(),
        &[1u8],
    ])
    .to_bytes()
}

fn hash_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    solana_sha256_hasher::hashv(&[
        b"bouncer:node".as_ref(),
        left.as_ref(),
        right.as_ref(),
    ])
    .to_bytes()
}

fn get_bit_le(bytes: &[u8; 32], bit_index: usize) -> bool {
    let byte = bytes[bit_index / 8];
    ((byte >> (bit_index % 8)) & 1) == 1
}

pub fn expected_leaf_value_for_policy(policy: u8) -> Result<u8> {
    match policy {
        POLICY_ALLOWLIST => Ok(1),
        POLICY_BLOCKLIST => Ok(0),
        _ => err!(BouncerError::InvalidPolicy),
    }
}

pub fn verify_sparse_merkle_proof(
    root: [u8; 32],
    depth: u8,
    key_hash: [u8; 32],
    expected_leaf_value: u8,
    proof: &[u8],
) -> Result<()> {
    require!(
        depth > 0 && depth <= MERKLE_MAX_DEPTH,
        BouncerError::InvalidMerkleConfig
    );
    require!(
        expected_leaf_value == 0 || expected_leaf_value == 1,
        BouncerError::InvalidMerkleConfig
    );

    let depth_usize = depth as usize;
    require!(
        proof.len() == depth_usize * 32,
        BouncerError::InvalidProofLength
    );

    let mut current = if expected_leaf_value == 0 {
        ZERO_LEAF
    } else {
        hash_leaf_one(&key_hash)
    };

    for i in 0..depth_usize {
        let mut sibling = [0u8; 32];
        sibling.copy_from_slice(&proof[i * 32..(i + 1) * 32]);

        let bit = get_bit_le(&key_hash, i);
        current = if bit {
            hash_node(&sibling, &current)
        } else {
            hash_node(&current, &sibling)
        };
    }

    require!(current == root, BouncerError::InvalidMerkleProof);
    Ok(())
}
