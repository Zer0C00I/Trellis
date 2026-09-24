//! Chain side of first-run family-member pairing: registers the family
//! member the kiosk just met over Bluetooth as this vault's first trusted
//! approver AND payee, signed by the device's own owner key.
//!
//! Idempotent by construction: it reads the on-chain state first and only
//! submits what is missing, all in ONE transaction (atomic — either every
//! missing step lands or none does), so a crash/retry mid-setup can never
//! double-add or leave a half-registered member.

use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use family_vault::state::{BeneficiaryList, TrustedApprovers};
use family_vault_client::{approvers_pda, beneficiary_list_pda, to_anchor, to_client_metas};
use solana_client::rpc_client::RpcClient;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

use crate::chain::{map_client_error_pub, ChainError};

#[derive(Debug, PartialEq, Eq)]
pub struct MemberSteps {
    pub init_approvers: bool,
    pub add_approver: bool,
    pub add_beneficiary: bool,
}

impl MemberSteps {
    pub fn is_empty(&self) -> bool {
        !self.init_approvers && !self.add_approver && !self.add_beneficiary
    }
}

pub struct MembershipState {
    /// `None` when the approvers account has not been created yet.
    pub approvers: Option<TrustedApprovers>,
    pub beneficiaries: BeneficiaryList,
}

pub struct MemberContext {
    pub program_id: Pubkey,
    pub owner: Pubkey,
    pub vault: Pubkey,
}

/// Pure: decides which of the three instructions are still needed.
pub fn plan_member_steps(
    approvers: Option<&TrustedApprovers>,
    beneficiaries: &BeneficiaryList,
    family: &Pubkey,
) -> MemberSteps {
    let family = to_anchor(*family);
    let add_approver = match approvers {
        Some(a) => !a.entries.iter().take(a.count as usize).any(|e| *e == family),
        None => true,
    };
    let add_beneficiary = !beneficiaries
        .entries
        .iter()
        .take(beneficiaries.count as usize)
        .any(|b| b.address == family);
    MemberSteps {
        init_approvers: approvers.is_none(),
        add_approver,
        add_beneficiary,
    }
}

/// Pairing is skipped on resume once someone is already an approver (the
/// transaction from a previous, interrupted setup attempt landed).
pub fn needs_pairing(state: &MembershipState) -> bool {
    match &state.approvers {
        None => true,
        Some(a) => a.count == 0,
    }
}

/// Builds the needed instructions in dependency order:
/// init approvers -> add approver -> add beneficiary.
pub fn build_member_instructions(
    ctx: &MemberContext,
    steps: &MemberSteps,
    family: &Pubkey,
    name: &str,
) -> Vec<Instruction> {
    let (approvers, _) = approvers_pda(&ctx.program_id, &ctx.vault);
    let (beneficiary_list, _) = beneficiary_list_pda(&ctx.program_id, &ctx.vault);
    let system_program = Pubkey::new_from_array([0u8; 32]);
    let mut ixs = Vec::new();

    if steps.init_approvers {
        let accounts = family_vault::accounts::InitializeApprovers {
            payer: to_anchor(ctx.owner),
            vault: to_anchor(ctx.vault),
            approvers: to_anchor(approvers),
            system_program: to_anchor(system_program),
        }
        .to_account_metas(None);
        ixs.push(Instruction {
            program_id: ctx.program_id,
            accounts: to_client_metas(&accounts),
            data: family_vault::instruction::InitializeApprovers {}.data(),
        });
    }

    if steps.add_approver {
        let accounts = family_vault::accounts::AddTrustedApprover {
            signer: to_anchor(ctx.owner),
            vault: to_anchor(ctx.vault),
            approvers: to_anchor(approvers),
            existing_approver: None,
        }
        .to_account_metas(None);
        ixs.push(Instruction {
            program_id: ctx.program_id,
            accounts: to_client_metas(&accounts),
            data: family_vault::instruction::AddTrustedApprover {
                address: to_anchor(*family),
            }
            .data(),
        });
    }

    if steps.add_beneficiary {
        let accounts = family_vault::accounts::AddBeneficiary {
            signer: to_anchor(ctx.owner),
            vault: to_anchor(ctx.vault),
            beneficiary_list: to_anchor(beneficiary_list),
        }
        .to_account_metas(None);
        ixs.push(Instruction {
            program_id: ctx.program_id,
            accounts: to_client_metas(&accounts),
            data: family_vault::instruction::AddBeneficiary {
                name: name.to_string(),
                address: to_anchor(*family),
            }
            .data(),
        });
    }

    ixs
}

/// Reads the approvers account (which may not exist yet) and the
/// beneficiary list in one RPC round trip. `get_multiple_accounts`
/// distinguishes "account does not exist" (`None`) from an RPC failure
/// (`Err`), which `get_account_data` cannot.
pub fn fetch_membership_state(
    rpc: &RpcClient,
    approvers: &Pubkey,
    beneficiary_list: &Pubkey,
) -> Result<MembershipState, ChainError> {
    let accounts = rpc
        .get_multiple_accounts(&[*approvers, *beneficiary_list])
        .map_err(map_client_error_pub)?;
    let [approvers_account, list_account] = accounts.as_slice() else {
        return Err(ChainError::ProgramRejected(
            "unexpected get_multiple_accounts response".to_string(),
        ));
    };

    let approvers = match approvers_account {
        Some(account) => Some(
            TrustedApprovers::try_deserialize(&mut &account.data[..])
                .map_err(|e| ChainError::ProgramRejected(e.to_string()))?,
        ),
        None => None,
    };
    let Some(list_account) = list_account else {
        return Err(ChainError::ProgramRejected(
            "beneficiary list account does not exist".to_string(),
        ));
    };
    let beneficiaries = BeneficiaryList::try_deserialize(&mut &list_account.data[..])
        .map_err(|e| ChainError::ProgramRejected(e.to_string()))?;

    Ok(MembershipState {
        approvers,
        beneficiaries,
    })
}

/// Registers `family` (named `name`) as approver and payee of `vault`,
/// submitting only what is still missing. Safe to call repeatedly.
pub fn add_first_family_member(
    rpc: &RpcClient,
    owner: &Keypair,
    program_id: &Pubkey,
    vault: &Pubkey,
    family: &Pubkey,
    name: &str,
) -> Result<(), ChainError> {
    let (approvers, _) = approvers_pda(program_id, vault);
    let (beneficiary_list, _) = beneficiary_list_pda(program_id, vault);

    let state = fetch_membership_state(rpc, &approvers, &beneficiary_list)?;
    let steps = plan_member_steps(state.approvers.as_ref(), &state.beneficiaries, family);
    if steps.is_empty() {
        return Ok(());
    }

    let ctx = MemberContext {
        program_id: *program_id,
        owner: owner.pubkey(),
        vault: *vault,
    };
    let ixs = build_member_instructions(&ctx, &steps, family, name);

    let blockhash = rpc.get_latest_blockhash().map_err(map_client_error_pub)?;
    let tx = solana_transaction::Transaction::new_signed_with_payer(
        &ixs,
        Some(&owner.pubkey()),
        &[owner],
        blockhash,
    );
    rpc.send_and_confirm_transaction(&tx)
        .map_err(map_client_error_pub)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::InstructionData;
    use family_vault::state::{Beneficiary, MAX_APPROVERS, MAX_BENEFICIARIES};
    use family_vault_client::AnchorPubkey;

    fn key(byte: u8) -> Pubkey {
        Pubkey::new_from_array([byte; 32])
    }

    fn approvers_with(members: &[Pubkey]) -> TrustedApprovers {
        let mut entries = [AnchorPubkey::default(); MAX_APPROVERS];
        for (i, m) in members.iter().enumerate() {
            entries[i] = to_anchor(*m);
        }
        TrustedApprovers {
            vault: Default::default(),
            count: members.len() as u8,
            entries,
        }
    }

    fn list_with(members: &[Pubkey]) -> BeneficiaryList {
        let mut entries = [Beneficiary::default(); MAX_BENEFICIARIES];
        for (i, m) in members.iter().enumerate() {
            entries[i].address = to_anchor(*m);
            entries[i].name_len = 1;
        }
        BeneficiaryList {
            vault: Default::default(),
            count: members.len() as u8,
            entries,
        }
    }

    fn ctx() -> MemberContext {
        MemberContext {
            program_id: key(1),
            owner: key(2),
            vault: key(3),
        }
    }

    #[test]
    fn plans_everything_when_nothing_exists_yet() {
        let steps = plan_member_steps(None, &list_with(&[]), &key(9));
        assert_eq!(
            steps,
            MemberSteps {
                init_approvers: true,
                add_approver: true,
                add_beneficiary: true
            }
        );
    }

    #[test]
    fn skips_init_when_the_approvers_account_exists() {
        let approvers = approvers_with(&[]);
        let steps = plan_member_steps(Some(&approvers), &list_with(&[]), &key(9));
        assert_eq!(
            steps,
            MemberSteps {
                init_approvers: false,
                add_approver: true,
                add_beneficiary: true
            }
        );
    }

    #[test]
    fn skips_whatever_is_already_done() {
        let approvers = approvers_with(&[key(9)]);
        let steps = plan_member_steps(Some(&approvers), &list_with(&[]), &key(9));
        assert_eq!(
            steps,
            MemberSteps {
                init_approvers: false,
                add_approver: false,
                add_beneficiary: true
            }
        );

        let approvers = approvers_with(&[key(9)]);
        let steps = plan_member_steps(Some(&approvers), &list_with(&[key(9)]), &key(9));
        assert!(steps.is_empty());
    }

    #[test]
    fn a_different_existing_member_does_not_count_as_done() {
        let approvers = approvers_with(&[key(4)]);
        let steps = plan_member_steps(Some(&approvers), &list_with(&[key(4)]), &key(9));
        assert!(steps.add_approver && steps.add_beneficiary && !steps.init_approvers);
    }

    #[test]
    fn pairing_is_needed_until_someone_is_an_approver() {
        let none = MembershipState {
            approvers: None,
            beneficiaries: list_with(&[]),
        };
        assert!(needs_pairing(&none));

        let empty = MembershipState {
            approvers: Some(approvers_with(&[])),
            beneficiaries: list_with(&[]),
        };
        assert!(needs_pairing(&empty));

        let done = MembershipState {
            approvers: Some(approvers_with(&[key(9)])),
            beneficiaries: list_with(&[key(9)]),
        };
        assert!(!needs_pairing(&done));
    }

    #[test]
    fn builds_all_three_instructions_in_dependency_order() {
        let family = key(9);
        let steps = MemberSteps {
            init_approvers: true,
            add_approver: true,
            add_beneficiary: true,
        };
        let ixs = build_member_instructions(&ctx(), &steps, &family, "Внук Саша");

        assert_eq!(ixs.len(), 3);
        assert_eq!(
            ixs[0].data,
            family_vault::instruction::InitializeApprovers {}.data()
        );
        assert_eq!(
            ixs[1].data,
            family_vault::instruction::AddTrustedApprover {
                address: to_anchor(family)
            }
            .data()
        );
        assert_eq!(
            ixs[2].data,
            family_vault::instruction::AddBeneficiary {
                name: "Внук Саша".to_string(),
                address: to_anchor(family)
            }
            .data()
        );
        for ix in &ixs {
            assert_eq!(ix.program_id, ctx().program_id);
        }
    }

    #[test]
    fn the_owner_signs_every_instruction() {
        let steps = MemberSteps {
            init_approvers: true,
            add_approver: true,
            add_beneficiary: true,
        };
        let ixs = build_member_instructions(&ctx(), &steps, &key(9), "Внук Саша");
        for ix in &ixs {
            assert!(
                ix.accounts
                    .iter()
                    .any(|m| m.pubkey == ctx().owner && m.is_signer),
                "owner must be a signer"
            );
        }
    }

    #[test]
    fn builds_only_the_needed_instructions() {
        let steps = MemberSteps {
            init_approvers: false,
            add_approver: false,
            add_beneficiary: true,
        };
        let ixs = build_member_instructions(&ctx(), &steps, &key(9), "Внук Саша");
        assert_eq!(ixs.len(), 1);
        assert_eq!(
            ixs[0].data,
            family_vault::instruction::AddBeneficiary {
                name: "Внук Саша".to_string(),
                address: to_anchor(key(9))
            }
            .data()
        );

        let none = MemberSteps {
            init_approvers: false,
            add_approver: false,
            add_beneficiary: false,
        };
        assert!(build_member_instructions(&ctx(), &none, &key(9), "x").is_empty());
    }
}
