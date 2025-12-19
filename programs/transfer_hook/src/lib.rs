use anchor_lang::{
    prelude::*, system_program::{CreateAccount, create_account}
};
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta, seeds::Seed, state::ExtraAccountMetaList,
};
use spl_transfer_hook_interface::instruction::{ExecuteInstruction, TransferHookInstruction};

mod account_structs;
mod helpers;
mod state;

use account_structs::*;
use helpers::*;


#[error_code]
pub enum ErrorCode {
    #[msg("Transfer not allowed")]
    TransferNotAllowed,
    #[msg("Custom error")]
    Custom,
}

declare_id!("ABaNE7SfMYnVpYPYeEt8KBEts2hY6RhrovoiA5wqzDaV");

#[program]
pub mod market_transfer_hook {
    use super::*;

    // ------------------------------------------------------------
    // Initialize ExtraAccountMetaList (ONCE per mint)
    // ------------------------------------------------------------
    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>,
    ) -> Result<()> {
        // We only need ONE extra account: config
        let metas = vec![
            ExtraAccountMeta::new_with_seeds(
                &[Seed::AccountKey { index: 0 }],
                false,
                false,
            ).map_err(to_anchor_error_tlv)?,
        ];

        let size = ExtraAccountMetaList::size_of(metas.len()).map_err(to_anchor_error_tlv)? as u64;
        let lamports = Rent::get()?.minimum_balance(size as usize);

        let mint = ctx.accounts.mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"extra-account-metas",
            mint.as_ref(),
            &[ctx.bumps.extra_account_meta_list],
        ]];

        create_account(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                CreateAccount {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.extra_account_meta_list.to_account_info(),
                },
            )
            .with_signer(signer_seeds),
            lamports,
            size,
            ctx.program_id,
        )?;

        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
            &metas,
        ).map_err(to_anchor_error_tlv)?;

        Ok(())
    }

    // ------------------------------------------------------------
    // Transfer hook (called on every transfer / transfer_checked)
    // ------------------------------------------------------------
    pub fn transfer_hook(ctx: Context<TransferHook>, _amount: u64) -> Result<()> {
        let source = &ctx.accounts.source_token;
        let destination = &ctx.accounts.destination_token;
        let mint = ctx.accounts.mint.key();
        let config = &ctx.accounts.config;

        // ---- classify source ----
        let src_is_user_ata = is_user_ata(
            source,
            &mint,
            ctx.accounts.token_program.key(),
        );

        let src_is_market_vault = is_market_vault(
            source,
            &mint,
            &config.market_program_id,
            ctx.accounts.token_program.key(),
        );

        // ---- classify destination ----
        let dst_is_user_ata = is_user_ata(
            destination,
            &mint,
            ctx.accounts.token_program.key(),
        );

        let dst_is_market_vault = is_market_vault(
            destination,
            &mint,
            &config.market_program_id,
            ctx.accounts.token_program.key(),
        );

        // ---- enforce rules ----
        let allowed =
            // user -> market
            (src_is_user_ata && dst_is_market_vault)
            // market -> user
            || (src_is_market_vault && dst_is_user_ata);

        require!(allowed, ErrorCode::TransferNotAllowed);

        Ok(())
    }

    // ------------------------------------------------------------
    // REQUIRED fallback for Anchor
    // ------------------------------------------------------------
    pub fn fallback<'info>(
        program_id: &Pubkey,
        accounts: &'info [AccountInfo<'info>],
        data: &[u8],
    ) -> Result<()> {
        let instruction = TransferHookInstruction::unpack(data).map_err(to_anchor_error_tlv)?;

        match instruction {
            TransferHookInstruction::Execute { amount } => {
                let amount_bytes = amount.to_le_bytes();
                __private::__global::transfer_hook(program_id, accounts, &amount_bytes)
            }
            _ => Err(ProgramError::InvalidInstructionData.into()),
        }
    }

    pub fn initialize_config(ctx: Context<InitializeConfig>, market_program_id: Pubkey) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.market_program_id = market_program_id;
        config.bump = ctx.bumps.config;
        Ok(())
    }

    pub fn update_config(ctx: Context<UpdateConfig>, market_program_id: Pubkey) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.market_program_id = market_program_id;
        Ok(())
    }
}