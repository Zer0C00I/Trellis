use anchor_lang::prelude::*;

pub const MAX_BENEFICIARIES: usize = 10;
// reserved for the family-vault-approvals follow-up plan
pub const MAX_APPROVERS: usize = 5;
pub const BENEFICIARY_NAME_LEN: usize = 32;
pub const BENEFICIARY_COOLDOWN_SECS: i64 = 172_800; // 48h

#[account]
pub struct Vault {
    pub owner: Pubkey,
    pub vault_token_account: Pubkey,
    pub daily_limit: u64,
    pub spent_today: u64,
    pub last_reset_day: i64,
    pub is_pool: bool,
    pub bump: u8,
}

impl Vault {
    pub const SPACE: usize = 8 // discriminator
        + 32 // owner
        + 32 // vault_token_account
        + 8  // daily_limit
        + 8  // spent_today
        + 8  // last_reset_day
        + 1  // is_pool
        + 1; // bump
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct Beneficiary {
    pub address: Pubkey,
    pub name: [u8; BENEFICIARY_NAME_LEN],
    pub name_len: u8,
    pub added_at: i64,
    pub active_after: i64,
}

impl Default for Beneficiary {
    fn default() -> Self {
        Beneficiary {
            address: Pubkey::default(),
            name: [0u8; BENEFICIARY_NAME_LEN],
            name_len: 0,
            added_at: 0,
            active_after: 0,
        }
    }
}

#[account]
pub struct BeneficiaryList {
    pub vault: Pubkey,
    pub count: u8,
    pub entries: [Beneficiary; MAX_BENEFICIARIES],
}

impl BeneficiaryList {
    pub const SPACE: usize = 8 // discriminator
        + 32 // vault
        + 1  // count
        + (32 + BENEFICIARY_NAME_LEN + 1 + 8 + 8) * MAX_BENEFICIARIES;
}

#[account]
pub struct TrustedApprovers {
    pub vault: Pubkey,
    pub count: u8,
    pub entries: [Pubkey; MAX_APPROVERS],
}

impl TrustedApprovers {
    pub const SPACE: usize = 8 // discriminator
        + 32 // vault
        + 1  // count
        + 32 * MAX_APPROVERS;
}

#[account]
pub struct PendingWithdrawal {
    pub vault: Pubkey,
    pub amount: u64,
    pub to: Pubkey,
    pub requested_at: i64,
    pub approved_by: Option<Pubkey>,
    pub executed: bool,
    pub bump: u8,
}

impl PendingWithdrawal {
    pub const SPACE: usize = 8 // discriminator
        + 32 // vault
        + 8  // amount
        + 32 // to
        + 8  // requested_at
        + (1 + 32) // approved_by: Option<Pubkey>
        + 1  // executed
        + 1; // bump
}

pub const ROTATION_TIMELOCK_SECS: i64 = 259_200; // 72h

#[account]
pub struct PendingRotation {
    pub vault: Pubkey,
    pub new_owner: Pubkey,
    pub requested_by: Pubkey,
    pub requested_at: i64,
    pub executes_at: i64,
    pub cancelled: bool,
    pub bump: u8,
}

impl PendingRotation {
    pub const SPACE: usize = 8 // discriminator
        + 32 // vault
        + 32 // new_owner
        + 32 // requested_by
        + 8  // requested_at
        + 8  // executes_at
        + 1  // cancelled
        + 1; // bump
}
