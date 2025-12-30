use anchor_lang::prelude::*;

use crate::{
    errors::BouncerError,
    state::{
        Entry, EntryRemoved, EntryUpserted, List, ENTRY_STATUS_ALLOW, ENTRY_STATUS_BLOCK,
        STORAGE_DIRECT_PDA, ENTRY_VERSION,
    },
};

#[derive(Accounts)]
#[instruction(subject: Pubkey)]
pub struct UpsertEntry<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority @ BouncerError::Unauthorized,
        constraint = !list.is_frozen() @ BouncerError::Frozen,
        constraint = list.storage_kind == STORAGE_DIRECT_PDA @ BouncerError::InvalidStorageKind,
        seeds = [b"bouncer", list.creator.as_ref(), &list.list_id.to_le_bytes()],
        bump = list.bump
    )]
    pub list: Account<'info, List>,

    #[account(
        init_if_needed,
        payer = payer,
        space = Entry::LEN,
        seeds = [b"entry", list.key().as_ref(), subject.as_ref()],
        bump
    )]
    pub entry: Account<'info, Entry>,

    pub system_program: Program<'info, System>,
}

pub fn upsert_entry(ctx: Context<UpsertEntry>, subject: Pubkey, status: u8) -> Result<()> {
    require!(
        status == ENTRY_STATUS_ALLOW || status == ENTRY_STATUS_BLOCK,
        BouncerError::InvalidStatus
    );

    let list = &mut ctx.accounts.list;
    let entry = &mut ctx.accounts.entry;

    let is_new = entry.version == 0;
    if is_new {
        entry.version = ENTRY_VERSION;
        entry.bump = ctx.bumps.entry;
        entry.list = list.key();
        entry.subject = subject;
        entry.reserved = 0;
        list.entry_count = list.entry_count.saturating_add(1);
    } else {
        require_keys_eq!(entry.list, list.key(), BouncerError::EntryDataMismatch);
        require_keys_eq!(entry.subject, subject, BouncerError::EntryDataMismatch);
    }

    entry.status = status;

    emit!(EntryUpserted {
        list: list.key(),
        subject,
        status
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(subject: Pubkey)]
pub struct RemoveEntry<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority @ BouncerError::Unauthorized,
        constraint = !list.is_frozen() @ BouncerError::Frozen,
        constraint = list.storage_kind == STORAGE_DIRECT_PDA @ BouncerError::InvalidStorageKind,
        seeds = [b"bouncer", list.creator.as_ref(), &list.list_id.to_le_bytes()],
        bump = list.bump
    )]
    pub list: Account<'info, List>,

    #[account(
        mut,
        close = refund_to,
        seeds = [b"entry", list.key().as_ref(), subject.as_ref()],
        bump = entry.bump
    )]
    pub entry: Account<'info, Entry>,

    /// CHECK: rent refund destination
    #[account(mut)]
    pub refund_to: UncheckedAccount<'info>,
}

pub fn remove_entry(ctx: Context<RemoveEntry>, subject: Pubkey) -> Result<()> {
    let list = &mut ctx.accounts.list;
    let entry = &ctx.accounts.entry;

    require_keys_eq!(entry.list, list.key(), BouncerError::EntryDataMismatch);
    require_keys_eq!(entry.subject, subject, BouncerError::EntryDataMismatch);

    list.entry_count = list.entry_count.saturating_sub(1);

    emit!(EntryRemoved {
        list: list.key(),
        subject
    });

    Ok(())
}

