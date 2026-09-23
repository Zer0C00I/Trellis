use anchor_lang::prelude::*;

pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("9ozRBpTmDJJzcmJhKEbfvsoe7qqo2584bus1rSMG4zwJ");

#[program]
pub mod family_vault {
    use super::*;

    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        is_pool: bool,
        daily_limit: u64,
    ) -> Result<()> {
        instructions::initialize_vault(ctx, is_pool, daily_limit)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit(ctx, amount)
    }

    pub fn add_beneficiary(
        ctx: Context<AddBeneficiary>,
        name: String,
        address: Pubkey,
    ) -> Result<()> {
        instructions::add_beneficiary(ctx, name, address)
    }

    pub fn add_beneficiary_urgent(
        ctx: Context<AddBeneficiaryUrgent>,
        name: String,
        address: Pubkey,
    ) -> Result<()> {
        instructions::add_beneficiary_urgent(ctx, name, address)
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

    pub fn request_withdrawal(
        ctx: Context<RequestWithdrawal>,
        amount: u64,
        nonce: u64,
    ) -> Result<()> {
        instructions::request_withdrawal(ctx, amount, nonce)
    }

    pub fn approve_withdrawal(ctx: Context<ApproveWithdrawal>) -> Result<()> {
        instructions::approve_withdrawal(ctx)
    }

    pub fn request_owner_rotation(
        ctx: Context<RequestOwnerRotation>,
        new_owner: Pubkey,
        nonce: u64,
    ) -> Result<()> {
        instructions::request_owner_rotation(ctx, new_owner, nonce)
    }

    pub fn cancel_owner_rotation(ctx: Context<CancelOwnerRotation>) -> Result<()> {
        instructions::cancel_owner_rotation(ctx)
    }

    pub fn execute_owner_rotation(ctx: Context<ExecuteOwnerRotation>) -> Result<()> {
        instructions::execute_owner_rotation(ctx)
    }
}
