use anchor_lang::prelude::*;

use crate::{
    errors::BouncerError,
    state::{List, MerkleConfigSet, MERKLE_MAX_DEPTH, STORAGE_MERKLE_ROOT},
};

#[derive(Accounts)]
pub struct SetMerkleConfig<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority @ BouncerError::Unauthorized,
        constraint = !list.is_frozen() @ BouncerError::Frozen,
        constraint = list.storage_kind == STORAGE_MERKLE_ROOT @ BouncerError::InvalidStorageKind,
        seeds = [b"bouncer", list.creator.as_ref(), &list.list_id.to_le_bytes()],
        bump = list.bump
    )]
    pub list: Account<'info, List>,
}

pub fn set_merkle_config(
    ctx: Context<SetMerkleConfig>,
    depth: u8,
    root: [u8; 32],
) -> Result<()> {
    require!(
        depth > 0 && depth <= MERKLE_MAX_DEPTH,
        BouncerError::InvalidMerkleConfig
    );

    let list = &mut ctx.accounts.list;
    list.set_merkle_config(depth, root);

    emit!(MerkleConfigSet {
        list: list.key(),
        authority: ctx.accounts.authority.key(),
        depth,
        root,
    });

    Ok(())
}

