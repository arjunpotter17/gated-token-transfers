use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub market_program_id: Pubkey,
    pub bump: u8,
}
