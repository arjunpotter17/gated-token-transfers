use anchor_lang::prelude::*;

pub mod errors;
pub mod instructions;
pub mod merkle;
pub mod state;

use instructions::*;

declare_id!("4qn7TjxgnALkV5wjqSjeedSPx8XbacSYNKH4Gv54QEQC");

#[program]
pub mod bouncer {
    use super::*;

    pub fn create_list(
        ctx: Context<CreateList>,
        list_id: u64,
        policy: u8,
        storage_kind: u8,
        flags: u16,
    ) -> Result<()> {
        instructions::create_list(ctx, list_id, policy, storage_kind, flags)
    }

    pub fn set_authority(ctx: Context<SetAuthority>, new_authority: Pubkey) -> Result<()> {
        instructions::set_authority(ctx, new_authority)
    }

    pub fn set_policy(ctx: Context<SetPolicy>, new_policy: u8) -> Result<()> {
        instructions::set_policy(ctx, new_policy)
    }

    pub fn freeze_list(ctx: Context<FreezeList>) -> Result<()> {
        instructions::freeze_list(ctx)
    }

    pub fn upsert_entry(
        ctx: Context<UpsertEntry>,
        subject: Pubkey,
        status: u8,
    ) -> Result<()> {
        instructions::upsert_entry(ctx, subject, status)
    }

    pub fn remove_entry(ctx: Context<RemoveEntry>, subject: Pubkey) -> Result<()> {
        instructions::remove_entry(ctx, subject)
    }

    pub fn assert_allowed(
        ctx: Context<AssertAllowed>,
        subject: Pubkey,
        proof: Vec<u8>,
    ) -> Result<()> {
        instructions::assert_allowed(ctx, subject, proof)
    }

    pub fn set_merkle_config(
        ctx: Context<SetMerkleConfig>,
        depth: u8,
        root: [u8; 32],
    ) -> Result<()> {
        instructions::set_merkle_config(ctx, depth, root)
    }
}
