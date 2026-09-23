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
    #[msg("An existing trusted approver must co-sign to add another approver to a personal vault")]
    ExistingApproverRequired,
    #[msg("Requested amount does not exceed the daily limit; use ordinary withdraw instead")]
    AmountDoesNotExceedDailyLimit,
    #[msg("The vault owner may not be added as a trusted approver")]
    CannotApproveOwnWithdrawals,
    #[msg("An approver may not name themselves as the new owner")]
    CannotRotateToSelf,
    #[msg("Approver address may not be the default (all-zeros) pubkey")]
    InvalidApproverAddress,
    #[msg("The two approvers co-signing an urgent beneficiary add must be different people")]
    DuplicateApprover,
}
