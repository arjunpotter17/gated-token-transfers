use anchor_lang::{
    prelude::*,
    system_program::{create_account, CreateAccount},
};
use spl_tlv_account_resolution::{account::ExtraAccountMeta, state::ExtraAccountMetaList};
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

declare_id!("EdB4jakxsXGit5ojRshNv2bgfNNKgo6zqM5FEWiNLvtR");

#[program]
pub mod market_transfer_hook {
    use super::*;

    // ------------------------------------------------------------
    // Initialize ExtraAccountMetaList (ONCE per mint)
    // ------------------------------------------------------------
    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>,
    ) -> Result<()> {
        // index 0-3 are the accounts required for token transfer (source, mint, destination, owner)
        // index 4 is address of ExtraAccountMetaList account
        // index 5 is address of Config account
        // index 6 is address of Bouncer list
        // index 7 is address of Token program
        let metas = vec![
            //index 5 = config account
            ExtraAccountMeta::new_with_pubkey(
                &spl_tlv_account_resolution::solana_pubkey::Pubkey::new_from_array(
                    ctx.accounts.config.key().to_bytes(),
                ),
                false,
                false,
            )
            .map_err(to_anchor_error_tlv)?,
            // index 6 = bouncer_list
            ExtraAccountMeta::new_with_pubkey(
                &spl_tlv_account_resolution::solana_pubkey::Pubkey::new_from_array(
                    ctx.accounts.bouncer_list.key().to_bytes(),
                ),
                false,
                false,
            )
            .map_err(to_anchor_error_tlv)?,
            // index 7 = bouncer_program
            ExtraAccountMeta::new_with_pubkey(
                &spl_tlv_account_resolution::solana_pubkey::Pubkey::new_from_array(
                    ctx.accounts.bouncer_program.key().to_bytes(),
                ),
                false,
                false,
            )
            .map_err(to_anchor_error_tlv)?,
            // index 8 = entry_account
            ExtraAccountMeta::new_with_pubkey(
                &spl_tlv_account_resolution::solana_pubkey::Pubkey::new_from_array(
                    ctx.accounts.entry_account.key().to_bytes(),
                ),
                false,
                false,
            )
            .map_err(to_anchor_error_tlv)?,
            // index 9 = token_program
            ExtraAccountMeta::new_with_pubkey(
                &spl_tlv_account_resolution::solana_pubkey::Pubkey::new_from_array(
                    ctx.accounts.token_program.key().to_bytes(),
                ),
                false,
                false,
            )
            .map_err(to_anchor_error_tlv)?,
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
        )
        .map_err(to_anchor_error_tlv)?;

        Ok(())
    }

    // ------------------------------------------------------------
    // Transfer hook (called on every transfer / transfer_checked)
    // ------------------------------------------------------------
    pub fn transfer_hook(ctx: Context<TransferHook>, _amount: u64) -> Result<()> {
        let src_owner = ctx.accounts.source_token.owner;
        let dst_owner = ctx.accounts.destination_token.owner;

        msg!("src_owner: {}", src_owner.to_string());
        msg!("dst_owner: {}", dst_owner.to_string());
        msg!("entry_account: {}", ctx.accounts.entry_account.key().to_string());
        msg!(
            "source_token: {}",
            ctx.accounts.source_token.key().to_string()
        );
        msg!(
            "destination_token: {}",
            ctx.accounts.destination_token.key().to_string()
        );
        msg!("mint: {}", ctx.accounts.mint.key().to_string());
        msg!(
            "token_program: {}",
            ctx.accounts.token_program.key().to_string()
        );
        msg!(
            "bouncer_program: {}",
            ctx.accounts.bouncer_program.key().to_string()
        );
        msg!(
            "bouncer_list: {}",
            ctx.accounts.bouncer_list.key().to_string()
        );

        // Check if destination is whitelisted
        check_whitelist(&ctx, dst_owner)?;

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


    //TODO - Need to add the assert from docs so that this is called only through a transfer_checked instruction
    //and not directly

    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        bouncer_program_id: Pubkey,
        bouncer_list: Pubkey,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.bouncer_program_id = bouncer_program_id;
        config.bouncer_list = bouncer_list;
        config.bump = ctx.bumps.config;
        Ok(())
    }

    pub fn update_config(
        ctx: Context<UpdateConfig>,
        bouncer_program_id: Option<Pubkey>,
        bouncer_list: Option<Pubkey>,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        if let Some(bouncer_program_id) = bouncer_program_id {
            config.bouncer_program_id = bouncer_program_id;
        }
        if let Some(bouncer_list) = bouncer_list {
            config.bouncer_list = bouncer_list;
        }
        Ok(())
    }

    pub fn close_config(ctx: Context<CloseConfig>) -> Result<()> {
        // Clear the account data and transfer excess lamports to payer
        // Keep only the rent-exempt amount so account remains valid
        // Then init_if_needed can reinitialize it with new structure
        let config_info = ctx.accounts.config.to_account_info();
        let payer_info = ctx.accounts.payer.to_account_info();

        // Calculate rent-exempt amount for the account size
        let account_size = config_info.data_len();
        let rent = Rent::get()?;
        let rent_exempt_amount = rent.minimum_balance(account_size);

        // Get current lamports
        let current_lamports = config_info.lamports();

        // Calculate excess lamports to transfer
        let excess_lamports = current_lamports
            .checked_sub(rent_exempt_amount)
            .unwrap_or(0);

        // Transfer excess lamports to payer
        if excess_lamports > 0 {
            **config_info.lamports.borrow_mut() = rent_exempt_amount;
            **payer_info.lamports.borrow_mut() = payer_info
                .lamports()
                .checked_add(excess_lamports)
                .ok_or(ErrorCode::Custom)?;
        }

        // Clear the account data (set to all zeros)
        // This removes the discriminator, allowing init_if_needed to reinitialize
        let mut data = config_info.try_borrow_mut_data()?;
        data.fill(0);

        // Account now has zeroed data but still has rent-exempt lamports
        // init_if_needed will see no discriminator and reinitialize it
        Ok(())
    }
}
