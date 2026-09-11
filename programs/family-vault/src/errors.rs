use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Only the vault owner may deposit into a personal vault")]
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
}
