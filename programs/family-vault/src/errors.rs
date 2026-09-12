use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Only the vault owner may perform this action")]
    NotVaultOwner,
    #[msg("Beneficiary list is full")]
    BeneficiaryListFull,
    #[msg("Beneficiary already exists")]
    BeneficiaryAlreadyExists,
    #[msg("Beneficiary name exceeds maximum length")]
    NameTooLong,
    #[msg("Recipient is not an approved beneficiary")]
    UnknownBeneficiary,
    #[msg("Beneficiary cooldown has not elapsed yet")]
    BeneficiaryCooldownActive,
    #[msg("Amount overflowed while adding to spent_today")]
    AmountOverflow,
    #[msg("Withdrawal would exceed the vault's daily limit")]
    DailyLimitExceeded,
    #[msg("Pool vault daily_limit must be greater than zero")]
    InvalidDailyLimit,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Trusted approver list is full")]
    ApproverListFull,
    #[msg("Address is already a trusted approver")]
    ApproverAlreadyExists,
    #[msg("This instruction is only valid for pool vaults")]
    NotPoolVault,
    #[msg("This withdrawal request has already been executed")]
    RequestAlreadyExecuted,
    #[msg("Signer is not a trusted approver")]
    NotTrustedApprover,
    #[msg("This rotation request has already been cancelled")]
    RotationAlreadyCancelled,
    #[msg("Only the vault owner or a trusted approver may cancel a rotation")]
    NotAuthorizedToCancel,
    #[msg("The rotation timelock has not elapsed yet")]
    RotationTimelockNotElapsed,
}
