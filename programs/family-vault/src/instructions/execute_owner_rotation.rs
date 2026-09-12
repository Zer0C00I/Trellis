use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::{PendingRotation, Vault};

pub fn execute_owner_rotation(ctx: Context<ExecuteOwnerRotation>) -> Result<()> {
    let rotation = &ctx.accounts.rotation;
    require!(!rotation.cancelled, VaultError::RotationAlreadyCancelled);
    require!(!rotation.executed, VaultError::RequestAlreadyExecuted);

    let now = Clock::get()?.unix_timestamp;
    require!(now >= rotation.executes_at, VaultError::RotationTimelockNotElapsed);

    let new_owner = rotation.new_owner;
    ctx.accounts.vault.owner = new_owner;
    ctx.accounts.rotation.executed = true;

    Ok(())
}

#[derive(Accounts)]
pub struct ExecuteOwnerRotation<'info> {
    // Permissionless: anyone may trigger execution once the timelock has
    // elapsed. `Signer` only proves this transaction was submitted with a
    // valid signature — it does not gate who that signer may be.
    pub signer: Signer<'info>,

    #[account(mut)]
    pub vault: Account<'info, Vault>,

    #[account(mut, has_one = vault)]
    pub rotation: Account<'info, PendingRotation>,
}
