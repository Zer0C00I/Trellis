use anchor_lang::prelude::*;

pub mod errors;
pub mod state;
pub mod instructions;

use instructions::*;

declare_id!("FVau1t1111111111111111111111111111111111111");

#[program]
pub mod family_vault {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>, is_pool: bool, daily_limit: u64) -> Result<()> {
        instructions::initialize_vault(ctx, is_pool, daily_limit)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit(ctx, amount)
    }
}
