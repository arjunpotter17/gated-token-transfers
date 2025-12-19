use anchor_lang::{prelude::*, solana_program};
use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token_interface::TokenAccount};

pub fn is_user_ata(
    token: &InterfaceAccount<TokenAccount>,
    mint: &Pubkey,
    token_program: Pubkey,
) -> bool {
    let expected = get_associated_token_address_with_program_id(
        &token.owner,
        mint,
        &token_program,
    );
    token.key() == expected
}

pub fn is_market_vault(
    token: &InterfaceAccount<TokenAccount>,
    mint: &Pubkey,
    market_program_id: &Pubkey,
    token_program: Pubkey,
) -> bool {
    // owner must be a PDA owned by market program
    if Pubkey::is_on_curve(&token.owner) {
        return false;
    }

    let expected = get_associated_token_address_with_program_id(
        &token.owner,
        mint,
        &token_program,
    );

    token.key() == expected
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