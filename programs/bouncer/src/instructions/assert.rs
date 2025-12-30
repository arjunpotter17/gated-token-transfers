use anchor_lang::prelude::*;

use crate::{
    errors::BouncerError,
    merkle::{expected_leaf_value_for_policy, verify_sparse_merkle_proof},
    state::{
        Entry, List, POLICY_ALLOWLIST, POLICY_BLOCKLIST, STORAGE_DIRECT_PDA, STORAGE_MERKLE_ROOT,
    },
};

#[derive(Accounts)]
pub struct AssertAllowed<'info> {
    #[account(
        seeds = [b"bouncer", list.creator.as_ref(), &list.list_id.to_le_bytes()],
        bump = list.bump
    )]
    pub list: Account<'info, List>,
}

pub fn assert_allowed(ctx: Context<AssertAllowed>, subject: Pubkey, proof: Vec<u8>) -> Result<()> {
    let list = &ctx.accounts.list;
    require!(
        list.policy == POLICY_ALLOWLIST || list.policy == POLICY_BLOCKLIST,
        BouncerError::InvalidPolicy
    );

    match list.storage_kind {
        STORAGE_DIRECT_PDA => {
            require!(proof.is_empty(), BouncerError::ProofNotSupported);

            let expected_entry = Pubkey::find_program_address(
                &[b"entry", list.key().as_ref(), subject.as_ref()],
                ctx.program_id,
            )
            .0;

            let mut status = 0u8;

            // Optional entry PDA is passed as remaining account 0.
            if let Some(entry_info) = ctx.remaining_accounts.first() {
                require_keys_eq!(entry_info.key(), expected_entry, BouncerError::EntryPdaMismatch);
                require_keys_eq!(
                    *entry_info.owner,
                    *ctx.program_id,
                    BouncerError::EntryAccountInvalidOwner
                );

                let data = entry_info.try_borrow_data()?;
                let mut data_slice: &[u8] = &data;
                let entry = Entry::try_deserialize(&mut data_slice)
                    .map_err(|_| error!(BouncerError::EntryDataMismatch))?;

                require_keys_eq!(entry.list, list.key(), BouncerError::EntryDataMismatch);
                require_keys_eq!(entry.subject, subject, BouncerError::EntryDataMismatch);

                status = entry.status;
            }

            let allowed = match list.policy {
                POLICY_ALLOWLIST => status == crate::state::ENTRY_STATUS_ALLOW,
                POLICY_BLOCKLIST => status != crate::state::ENTRY_STATUS_BLOCK,
                _ => return err!(BouncerError::InvalidPolicy),
            };

            require!(allowed, BouncerError::NotAllowed);
            Ok(())
        }
        STORAGE_MERKLE_ROOT => {
            let depth = list.merkle_depth();
            let root = list.merkle_root();
            let expected_leaf_value = expected_leaf_value_for_policy(list.policy)?;
            let key_hash = solana_sha256_hasher::hash(subject.as_ref()).to_bytes();
            verify_sparse_merkle_proof(root, depth, key_hash, expected_leaf_value, &proof)?;
            Ok(())
        }
        _ => err!(BouncerError::InvalidStorageKind),
    }?;
    Ok(())
}
