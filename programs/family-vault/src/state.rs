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
