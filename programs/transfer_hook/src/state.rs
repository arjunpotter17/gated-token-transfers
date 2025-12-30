use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub bouncer_program_id: Pubkey,
    pub bouncer_list: Pubkey,
    pub bump: u8,
}
