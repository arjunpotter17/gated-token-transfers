use anchor_lang::{prelude::*, solana_program};

use crate::account_structs::TransferHook;
use crate::ErrorCode;

pub fn check_whitelist(
    ctx: &Context<TransferHook>,
    key: Pubkey,
) -> Result<()> {
    // Validate that bouncer_program and bouncer_list match config
    require_keys_eq!(
        ctx.accounts.bouncer_program.key(),
        ctx.accounts.config.bouncer_program_id,
        ErrorCode::TransferNotAllowed
    );
    require_keys_eq!(
        ctx.accounts.bouncer_list.key(),
        ctx.accounts.config.bouncer_list,
        ErrorCode::TransferNotAllowed
    );
    // Ensure bouncer_program is executable
    require!(
        ctx.accounts.bouncer_program.executable,
        ErrorCode::TransferNotAllowed
    );
    
    let cpi_ctx = CpiContext::new(
        ctx.accounts.bouncer_program.to_account_info(),
        bouncer::cpi::accounts::AssertAllowed {
            list: ctx.accounts.bouncer_list.to_account_info(),
        },
    );
    
    // Call bouncer and convert any error to TransferNotAllowed
    bouncer::cpi::assert_allowed(cpi_ctx, key, Vec::new())
        .map_err(|_| ErrorCode::TransferNotAllowed)?;
    
    Ok(())
}


// Helper to convert spl_tlv_account_resolution::solana_program_error::ProgramError to Anchor Error
pub fn to_anchor_error_tlv(err: spl_tlv_account_resolution::solana_program_error::ProgramError) -> Error {
    let solana_err = match err {
        spl_tlv_account_resolution::solana_program_error::ProgramError::Custom(code) => {
            solana_program::program_error::ProgramError::Custom(code)
        }
        _ => solana_program::program_error::ProgramError::InvalidInstructionData,
    };
    Error::from(ProgramError::from(solana_err))
}