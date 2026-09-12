use anchor_lang::prelude::*;

pub mod errors;
pub mod state;
pub mod instructions;

use instructions::*;

declare_id!("FXFxTexyhpdjmb6s2kvjVeY2aJFD9jmW8PsfSUY6LF7m");

#[program]
pub mod family_vault {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>, is_pool: bool, daily_limit: u64) -> Result<()> {
        instructions::initialize_vault(ctx, is_pool, daily_limit)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit(ctx, amount)
    }

    pub fn add_beneficiary(ctx: Context<AddBeneficiary>, name: String, address: Pubkey) -> Result<()> {
        instructions::add_beneficiary(ctx, name, address)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        instructions::withdraw(ctx, amount)
    }

    pub fn initialize_approvers(ctx: Context<InitializeApprovers>) -> Result<()> {
        instructions::initialize_approvers(ctx)
    }

    pub fn add_trusted_approver(ctx: Context<AddTrustedApprover>, address: Pubkey) -> Result<()> {
        instructions::add_trusted_approver(ctx, address)
    }
}
