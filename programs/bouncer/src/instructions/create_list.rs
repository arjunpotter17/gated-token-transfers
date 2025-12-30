use anchor_lang::prelude::*;

use crate::{
    errors::BouncerError,
    state::{
        List, ListCreated, POLICY_ALLOWLIST, POLICY_BLOCKLIST, STORAGE_DIRECT_PDA,
        STORAGE_MERKLE_ROOT, LIST_VERSION,
    },
};

#[derive(Accounts)]
#[instruction(list_id: u64)]
pub struct CreateList<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub authority: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = List::LEN,
        seeds = [b"bouncer", payer.key().as_ref(), &list_id.to_le_bytes()],
        bump
    )]
    pub list: Account<'info, List>,

    pub system_program: Program<'info, System>,
}

pub fn create_list(
    ctx: Context<CreateList>,
    list_id: u64,
    policy: u8,
    storage_kind: u8,
    flags: u16,
) -> Result<()> {
    require!(
        policy == POLICY_ALLOWLIST || policy == POLICY_BLOCKLIST,
        BouncerError::InvalidPolicy
    );
    require!(
        storage_kind == STORAGE_DIRECT_PDA || storage_kind == STORAGE_MERKLE_ROOT,
        BouncerError::InvalidStorageKind
    );

    let list = &mut ctx.accounts.list;
    list.version = LIST_VERSION;
    list.bump = ctx.bumps.list;
    list.authority = ctx.accounts.authority.key();
    list.creator = ctx.accounts.payer.key();
    list.list_id = list_id;
    list.policy = policy;
    list.storage_kind = storage_kind;
    list.flags = flags;
    list.entry_count = 0;
    list.reserved0 = 0;
    list.storage_config = [0u8; 96];

    emit!(ListCreated {
        list: list.key(),
        creator: list.creator,
        authority: list.authority,
        list_id,
        policy,
        storage_kind,
    });

    Ok(())
}
