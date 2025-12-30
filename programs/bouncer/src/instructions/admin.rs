use anchor_lang::prelude::*;

use crate::{
    errors::BouncerError,
    state::{
        List, AuthorityChanged, PolicyChanged, ListFrozen, FLAG_FROZEN, POLICY_ALLOWLIST,
        POLICY_BLOCKLIST,
    },
};

#[derive(Accounts)]
pub struct SetAuthority<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority @ BouncerError::Unauthorized,
        constraint = !list.is_frozen() @ BouncerError::Frozen,
        seeds = [b"bouncer", list.creator.as_ref(), &list.list_id.to_le_bytes()],
        bump = list.bump
    )]
    pub list: Account<'info, List>,
}

pub fn set_authority(ctx: Context<SetAuthority>, new_authority: Pubkey) -> Result<()> {
    let list = &mut ctx.accounts.list;
    let old = list.authority;
    list.authority = new_authority;
    emit!(AuthorityChanged {
        list: list.key(),
        old_authority: old,
        new_authority,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct SetPolicy<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority @ BouncerError::Unauthorized,
        constraint = !list.is_frozen() @ BouncerError::Frozen,
        seeds = [b"bouncer", list.creator.as_ref(), &list.list_id.to_le_bytes()],
        bump = list.bump
    )]
    pub list: Account<'info, List>,
}

pub fn set_policy(ctx: Context<SetPolicy>, new_policy: u8) -> Result<()> {
    require!(
        new_policy == POLICY_ALLOWLIST || new_policy == POLICY_BLOCKLIST,
        BouncerError::InvalidPolicy
    );
    let list = &mut ctx.accounts.list;
    let old = list.policy;
    list.policy = new_policy;
    emit!(PolicyChanged {
        list: list.key(),
        old_policy: old,
        new_policy,
    });
    Ok(())
}

#[derive(Accounts)]
pub struct FreezeList<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority @ BouncerError::Unauthorized,
        constraint = !list.is_frozen() @ BouncerError::Frozen,
        seeds = [b"bouncer", list.creator.as_ref(), &list.list_id.to_le_bytes()],
        bump = list.bump
    )]
    pub list: Account<'info, List>,
}

pub fn freeze_list(ctx: Context<FreezeList>) -> Result<()> {
    let list = &mut ctx.accounts.list;
    list.flags |= FLAG_FROZEN;
    emit!(ListFrozen { list: list.key() });
    Ok(())
}

