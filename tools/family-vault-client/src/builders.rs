//! Pure instruction builders for every `family-vault` operation a
//! bank-style client needs (open an account, deposit, send, add a payee,
//! add one urgently, add a trusted person, request/approve an over-limit
//! withdrawal). Each function only builds a `solana_instruction::
//! Instruction` — it never talks to an RPC and never signs anything, so
//! callers (currently `family-app-core`; `admin-cli` and `kiosk-gui`
//! predate this module and keep their own inline copies for now) are free
//! to batch, simulate, or hand these to a wallet for signing.
//!
//! Owner-rotation instructions (`request`/`cancel`/`execute_owner_rotation`)
//! are deliberately not here — out of scope for the family-app MVP (see
//! the family-app design spec).

use anchor_lang::{InstructionData, ToAccountMetas};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::{
    approvers_pda, beneficiary_list_pda, request_pda, to_anchor, to_client_metas, vault_pda,
};

fn system_program_id() -> Pubkey {
    Pubkey::new_from_array([0u8; 32])
}

pub fn build_initialize_vault_instruction(
    program_id: &Pubkey,
    payer: &Pubkey,
    owner: &Pubkey,
    mint: &Pubkey,
    is_pool: bool,
    daily_limit: u64,
) -> Instruction {
    let (vault, _) = vault_pda(program_id, owner, is_pool);
    let (beneficiary_list, _) = beneficiary_list_pda(program_id, &vault);
    let vault_token_account =
        spl_associated_token_account::get_associated_token_address(&vault, mint);

    let accounts = family_vault::accounts::InitializeVault {
        payer: to_anchor(*payer),
        owner: to_anchor(*owner),
        vault: to_anchor(vault),
        beneficiary_list: to_anchor(beneficiary_list),
        mint: to_anchor(*mint),
        vault_token_account: to_anchor(vault_token_account),
        token_program: to_anchor(spl_token::ID),
        associated_token_program: to_anchor(spl_associated_token_account::ID),
        system_program: to_anchor(system_program_id()),
    }
    .to_account_metas(None);

    Instruction {
        program_id: *program_id,
        accounts: to_client_metas(&accounts),
        data: family_vault::instruction::InitializeVault {
            is_pool,
            daily_limit,
        }
        .data(),
    }
}

pub fn build_initialize_approvers_instruction(
    program_id: &Pubkey,
    payer: &Pubkey,
    vault: &Pubkey,
) -> Instruction {
    let (approvers, _) = approvers_pda(program_id, vault);
    let accounts = family_vault::accounts::InitializeApprovers {
        payer: to_anchor(*payer),
        vault: to_anchor(*vault),
        approvers: to_anchor(approvers),
        system_program: to_anchor(system_program_id()),
    }
    .to_account_metas(None);

    Instruction {
        program_id: *program_id,
        accounts: to_client_metas(&accounts),
        data: family_vault::instruction::InitializeApprovers {}.data(),
    }
}

pub fn build_add_trusted_approver_instruction(
    program_id: &Pubkey,
    signer: &Pubkey,
    vault: &Pubkey,
    address: &Pubkey,
    existing_approver: Option<Pubkey>,
) -> Instruction {
    let (approvers, _) = approvers_pda(program_id, vault);
    let accounts = family_vault::accounts::AddTrustedApprover {
        signer: to_anchor(*signer),
        vault: to_anchor(*vault),
        approvers: to_anchor(approvers),
        existing_approver: existing_approver.map(to_anchor),
    }
    .to_account_metas(None);

    Instruction {
        program_id: *program_id,
        accounts: to_client_metas(&accounts),
        data: family_vault::instruction::AddTrustedApprover {
            address: to_anchor(*address),
        }
        .data(),
    }
}

pub fn build_add_beneficiary_instruction(
    program_id: &Pubkey,
    signer: &Pubkey,
    vault: &Pubkey,
    name: &str,
    address: &Pubkey,
) -> Instruction {
    let (beneficiary_list, _) = beneficiary_list_pda(program_id, vault);
    let accounts = family_vault::accounts::AddBeneficiary {
        signer: to_anchor(*signer),
        vault: to_anchor(*vault),
        beneficiary_list: to_anchor(beneficiary_list),
    }
    .to_account_metas(None);

    Instruction {
        program_id: *program_id,
        accounts: to_client_metas(&accounts),
        data: family_vault::instruction::AddBeneficiary {
            name: name.to_string(),
            address: to_anchor(*address),
        }
        .data(),
    }
}

pub fn build_add_beneficiary_urgent_instruction(
    program_id: &Pubkey,
    approver_one: &Pubkey,
    approver_two: &Pubkey,
    vault: &Pubkey,
    name: &str,
    address: &Pubkey,
) -> Instruction {
    let (approvers, _) = approvers_pda(program_id, vault);
    let (beneficiary_list, _) = beneficiary_list_pda(program_id, vault);
    let accounts = family_vault::accounts::AddBeneficiaryUrgent {
        approver_one: to_anchor(*approver_one),
        approver_two: to_anchor(*approver_two),
        vault: to_anchor(*vault),
        approvers: to_anchor(approvers),
        beneficiary_list: to_anchor(beneficiary_list),
    }
    .to_account_metas(None);

    Instruction {
        program_id: *program_id,
        accounts: to_client_metas(&accounts),
        data: family_vault::instruction::AddBeneficiaryUrgent {
            name: name.to_string(),
            address: to_anchor(*address),
        }
        .data(),
    }
}

pub fn build_deposit_instruction(
    program_id: &Pubkey,
    depositor: &Pubkey,
    vault: &Pubkey,
    vault_token_account: &Pubkey,
    mint: &Pubkey,
    amount: u64,
) -> Instruction {
    let depositor_token_account =
        spl_associated_token_account::get_associated_token_address(depositor, mint);
    let accounts = family_vault::accounts::Deposit {
        depositor: to_anchor(*depositor),
        depositor_token_account: to_anchor(depositor_token_account),
        vault_token_account: to_anchor(*vault_token_account),
        vault: to_anchor(*vault),
        token_program: to_anchor(spl_token::ID),
    }
    .to_account_metas(None);

    Instruction {
        program_id: *program_id,
        accounts: to_client_metas(&accounts),
        data: family_vault::instruction::Deposit { amount }.data(),
    }
}

pub fn build_withdraw_instruction(
    program_id: &Pubkey,
    signer: &Pubkey,
    vault: &Pubkey,
    vault_token_account: &Pubkey,
    mint: &Pubkey,
    recipient: &Pubkey,
    amount: u64,
) -> Instruction {
    let recipient_token_account =
        spl_associated_token_account::get_associated_token_address(recipient, mint);
    let (beneficiary_list, _) = beneficiary_list_pda(program_id, vault);
    let accounts = family_vault::accounts::Withdraw {
        signer: to_anchor(*signer),
        vault_token_account: to_anchor(*vault_token_account),
        recipient_token_account: to_anchor(recipient_token_account),
        vault: to_anchor(*vault),
        beneficiary_list: to_anchor(beneficiary_list),
        token_program: to_anchor(spl_token::ID),
    }
    .to_account_metas(None);

    Instruction {
        program_id: *program_id,
        accounts: to_client_metas(&accounts),
        data: family_vault::instruction::Withdraw { amount }.data(),
    }
}

pub fn build_request_withdrawal_instruction(
    program_id: &Pubkey,
    signer: &Pubkey,
    vault: &Pubkey,
    mint: &Pubkey,
    recipient: &Pubkey,
    amount: u64,
    nonce: u64,
) -> Instruction {
    let recipient_token_account =
        spl_associated_token_account::get_associated_token_address(recipient, mint);
    let (beneficiary_list, _) = beneficiary_list_pda(program_id, vault);
    let (pending_withdrawal, _) = request_pda(program_id, vault, nonce);
    let accounts = family_vault::accounts::RequestWithdrawal {
        signer: to_anchor(*signer),
        vault: to_anchor(*vault),
        beneficiary_list: to_anchor(beneficiary_list),
        recipient_token_account: to_anchor(recipient_token_account),
        pending_withdrawal: to_anchor(pending_withdrawal),
        system_program: to_anchor(system_program_id()),
    }
    .to_account_metas(None);

    Instruction {
        program_id: *program_id,
        accounts: to_client_metas(&accounts),
        data: family_vault::instruction::RequestWithdrawal { amount, nonce }.data(),
    }
}

pub fn build_approve_withdrawal_instruction(
    program_id: &Pubkey,
    signer: &Pubkey,
    vault: &Pubkey,
    vault_token_account: &Pubkey,
    mint: &Pubkey,
    recipient: &Pubkey,
    nonce: u64,
) -> Instruction {
    let recipient_token_account =
        spl_associated_token_account::get_associated_token_address(recipient, mint);
    let (approvers, _) = approvers_pda(program_id, vault);
    let (pending_withdrawal, _) = request_pda(program_id, vault, nonce);
    let accounts = family_vault::accounts::ApproveWithdrawal {
        signer: to_anchor(*signer),
        pending_withdrawal: to_anchor(pending_withdrawal),
        approvers: to_anchor(approvers),
        vault_token_account: to_anchor(*vault_token_account),
        recipient_token_account: to_anchor(recipient_token_account),
        vault: to_anchor(*vault),
        token_program: to_anchor(spl_token::ID),
    }
    .to_account_metas(None);

    Instruction {
        program_id: *program_id,
        accounts: to_client_metas(&accounts),
        data: family_vault::instruction::ApproveWithdrawal {}.data(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::InstructionData;

    fn key(byte: u8) -> Pubkey {
        Pubkey::new_from_array([byte; 32])
    }

    fn program_id() -> Pubkey {
        key(1)
    }

    #[test]
    fn initialize_vault_matches_the_program_id_and_encoding() {
        let ix = build_initialize_vault_instruction(
            &program_id(),
            &key(2), // payer
            &key(3), // owner
            &key(4), // mint
            true,
            1_000,
        );
        assert_eq!(ix.program_id, program_id());
        assert_eq!(
            ix.data,
            family_vault::instruction::InitializeVault {
                is_pool: true,
                daily_limit: 1_000,
            }
            .data()
        );
        // payer and owner must both sign; the vault/beneficiary_list/ATA
        // accounts are derived, not passed in, so this also proves the
        // builder computed them rather than requiring the caller to.
        assert!(ix
            .accounts
            .iter()
            .any(|m| m.pubkey == key(2) && m.is_signer));
        assert!(ix
            .accounts
            .iter()
            .any(|m| m.pubkey == key(3) && m.is_signer));
    }

    #[test]
    fn initialize_vault_derives_different_vaults_for_pool_and_personal() {
        let pool =
            build_initialize_vault_instruction(&program_id(), &key(2), &key(3), &key(4), true, 0);
        let personal =
            build_initialize_vault_instruction(&program_id(), &key(2), &key(3), &key(4), false, 0);
        let (pool_vault, _) = vault_pda(&program_id(), &key(3), true);
        let (personal_vault, _) = vault_pda(&program_id(), &key(3), false);
        assert!(pool.accounts.iter().any(|m| m.pubkey == pool_vault));
        assert!(personal.accounts.iter().any(|m| m.pubkey == personal_vault));
        assert_ne!(pool_vault, personal_vault);
    }

    #[test]
    fn initialize_approvers_requires_only_payer_to_sign() {
        let vault = key(3);
        let ix = build_initialize_approvers_instruction(&program_id(), &key(2), &vault);
        assert_eq!(
            ix.data,
            family_vault::instruction::InitializeApprovers {}.data()
        );
        let (approvers, _) = approvers_pda(&program_id(), &vault);
        assert!(ix
            .accounts
            .iter()
            .any(|m| m.pubkey == key(2) && m.is_signer));
        assert!(ix.accounts.iter().any(|m| m.pubkey == approvers));
    }

    #[test]
    fn add_trusted_approver_omits_existing_approver_when_none() {
        let vault = key(3);
        let ix =
            build_add_trusted_approver_instruction(&program_id(), &key(2), &vault, &key(9), None);
        assert_eq!(
            ix.data,
            family_vault::instruction::AddTrustedApprover {
                address: to_anchor(key(9)),
            }
            .data()
        );
        // Anchor's `Option<Signer>` still emits an account slot even when
        // `None` (as the program-id itself, a non-signer placeholder) —
        // assert there's no SECOND signer account beyond `signer`, since
        // that's the actual property that matters (whether the co-signer
        // is required), not the slot's mere presence.
        assert_eq!(ix.accounts.iter().filter(|m| m.is_signer).count(), 1);
    }

    #[test]
    fn add_trusted_approver_includes_existing_approver_as_a_signer_when_given() {
        let vault = key(3);
        let ix = build_add_trusted_approver_instruction(
            &program_id(),
            &key(2),
            &vault,
            &key(9),
            Some(key(5)),
        );
        assert!(ix
            .accounts
            .iter()
            .any(|m| m.pubkey == key(5) && m.is_signer));
        assert_eq!(ix.accounts.iter().filter(|m| m.is_signer).count(), 2);
    }

    #[test]
    fn add_beneficiary_matches_the_program_encoding() {
        let vault = key(3);
        let ix =
            build_add_beneficiary_instruction(&program_id(), &key(2), &vault, "Внук Саша", &key(9));
        assert_eq!(
            ix.data,
            family_vault::instruction::AddBeneficiary {
                name: "Внук Саша".to_string(),
                address: to_anchor(key(9)),
            }
            .data()
        );
        let (beneficiary_list, _) = beneficiary_list_pda(&program_id(), &vault);
        assert!(ix.accounts.iter().any(|m| m.pubkey == beneficiary_list));
    }

    #[test]
    fn add_beneficiary_urgent_requires_both_approvers_to_sign() {
        let vault = key(3);
        let ix = build_add_beneficiary_urgent_instruction(
            &program_id(),
            &key(6),
            &key(7),
            &vault,
            "Внук Саша",
            &key(9),
        );
        assert_eq!(
            ix.data,
            family_vault::instruction::AddBeneficiaryUrgent {
                name: "Внук Саша".to_string(),
                address: to_anchor(key(9)),
            }
            .data()
        );
        assert!(ix
            .accounts
            .iter()
            .any(|m| m.pubkey == key(6) && m.is_signer));
        assert!(ix
            .accounts
            .iter()
            .any(|m| m.pubkey == key(7) && m.is_signer));
    }

    #[test]
    fn deposit_derives_the_depositor_token_account_from_the_mint() {
        let vault = key(3);
        let vault_token_account = key(10);
        let mint = key(4);
        let ix = build_deposit_instruction(
            &program_id(),
            &key(2),
            &vault,
            &vault_token_account,
            &mint,
            500,
        );
        assert_eq!(
            ix.data,
            family_vault::instruction::Deposit { amount: 500 }.data()
        );
        let depositor_ata =
            spl_associated_token_account::get_associated_token_address(&key(2), &mint);
        assert!(ix.accounts.iter().any(|m| m.pubkey == depositor_ata));
        assert!(ix.accounts.iter().any(|m| m.pubkey == vault_token_account));
    }

    #[test]
    fn withdraw_derives_the_recipient_token_account_from_the_mint() {
        let vault = key(3);
        let vault_token_account = key(10);
        let mint = key(4);
        let recipient = key(9);
        let ix = build_withdraw_instruction(
            &program_id(),
            &key(2),
            &vault,
            &vault_token_account,
            &mint,
            &recipient,
            250,
        );
        assert_eq!(
            ix.data,
            family_vault::instruction::Withdraw { amount: 250 }.data()
        );
        let recipient_ata =
            spl_associated_token_account::get_associated_token_address(&recipient, &mint);
        assert!(ix.accounts.iter().any(|m| m.pubkey == recipient_ata));
        let (beneficiary_list, _) = beneficiary_list_pda(&program_id(), &vault);
        assert!(ix.accounts.iter().any(|m| m.pubkey == beneficiary_list));
    }

    #[test]
    fn request_withdrawal_derives_a_different_pending_withdrawal_per_nonce() {
        let vault = key(3);
        let mint = key(4);
        let recipient = key(9);
        let a = build_request_withdrawal_instruction(
            &program_id(),
            &key(2),
            &vault,
            &mint,
            &recipient,
            100,
            0,
        );
        let b = build_request_withdrawal_instruction(
            &program_id(),
            &key(2),
            &vault,
            &mint,
            &recipient,
            100,
            1,
        );
        assert_eq!(
            a.data,
            family_vault::instruction::RequestWithdrawal {
                amount: 100,
                nonce: 0
            }
            .data()
        );
        let (pending_a, _) = request_pda(&program_id(), &vault, 0);
        let (pending_b, _) = request_pda(&program_id(), &vault, 1);
        assert!(a.accounts.iter().any(|m| m.pubkey == pending_a));
        assert!(b.accounts.iter().any(|m| m.pubkey == pending_b));
        assert_ne!(pending_a, pending_b);
    }

    #[test]
    fn approve_withdrawal_targets_the_same_pending_withdrawal_as_the_request() {
        let vault = key(3);
        let vault_token_account = key(10);
        let mint = key(4);
        let recipient = key(9);
        let ix = build_approve_withdrawal_instruction(
            &program_id(),
            &key(2),
            &vault,
            &vault_token_account,
            &mint,
            &recipient,
            7,
        );
        assert_eq!(
            ix.data,
            family_vault::instruction::ApproveWithdrawal {}.data()
        );
        let (pending, _) = request_pda(&program_id(), &vault, 7);
        let (approvers, _) = approvers_pda(&program_id(), &vault);
        assert!(ix.accounts.iter().any(|m| m.pubkey == pending));
        assert!(ix.accounts.iter().any(|m| m.pubkey == approvers));
        let recipient_ata =
            spl_associated_token_account::get_associated_token_address(&recipient, &mint);
        assert!(ix.accounts.iter().any(|m| m.pubkey == recipient_ata));
    }
}
