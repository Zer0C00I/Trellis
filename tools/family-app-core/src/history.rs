//! Read-only transaction history for a vault: what happened, in plain
//! terms, without exposing raw instruction bytes to a caller. Two RPC
//! calls per signature (`getSignaturesForAddress` then `getTransaction`)
//! — a completely different Solana RPC surface than `discovery`'s
//! `getProgramAccounts` scan, since history lives in the ledger, not in
//! any account's current state.

use crate::error::{map_client_error, ActionError};
use anchor_lang::{AnchorDeserialize, InstructionData};
use family_vault::instruction::{
    AddBeneficiary, AddBeneficiaryUrgent, AddTrustedApprover, ApproveWithdrawal, Deposit,
    InitializeApprovers, InitializeVault, RequestWithdrawal, Withdraw,
};
use solana_client::rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient};
use solana_client::rpc_config::RpcTransactionConfig;
use solana_pubkey::Pubkey;
use solana_transaction_status_client_types::{
    EncodedTransaction, UiMessage, UiTransactionEncoding,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub signature: String,
    pub block_time: Option<i64>,
    pub kind: HistoryEntryKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryEntryKind {
    Deposit { amount: u64 },
    Withdraw { amount: u64 },
    AddBeneficiary { name: String },
    AddBeneficiaryUrgent { name: String },
    AddTrustedApprover,
    RequestWithdrawal { amount: u64, nonce: u64 },
    ApproveWithdrawal,
    InitializeVault,
    InitializeApprovers,
    Unknown,
}

/// The first 8 bytes of any Anchor instruction's encoded data are its
/// discriminator, fixed per instruction type regardless of argument
/// values — `disc_of` reads it from a throwaway instance purely to learn
/// that prefix, never to inspect the (irrelevant) dummy values.
fn disc_of(data: Vec<u8>) -> [u8; 8] {
    let mut d = [0u8; 8];
    d.copy_from_slice(&data[..8]);
    d
}

/// Decodes one instruction's raw bytes into a `HistoryEntryKind`. A
/// transaction touching this vault can legitimately contain other
/// programs' instructions too (e.g. the System Program's account-creation
/// instruction alongside `InitializeVault`) or a corrupt/short payload —
/// both decode as `Unknown` rather than an error, so one bad instruction
/// never fails the whole history fetch.
pub fn decode_instruction_kind(data: &[u8]) -> HistoryEntryKind {
    if data.len() < 8 {
        return HistoryEntryKind::Unknown;
    }
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&data[..8]);
    let payload = &data[8..];

    if disc
        == disc_of(
            InitializeVault {
                is_pool: false,
                daily_limit: 0,
            }
            .data(),
        )
    {
        return HistoryEntryKind::InitializeVault;
    }
    if disc == disc_of(InitializeApprovers {}.data()) {
        return HistoryEntryKind::InitializeApprovers;
    }
    if disc
        == disc_of(
            AddTrustedApprover {
                address: Default::default(),
            }
            .data(),
        )
    {
        return HistoryEntryKind::AddTrustedApprover;
    }
    if disc == disc_of(Deposit { amount: 0 }.data()) {
        return Deposit::try_from_slice(payload)
            .map(|ix| HistoryEntryKind::Deposit { amount: ix.amount })
            .unwrap_or(HistoryEntryKind::Unknown);
    }
    if disc == disc_of(Withdraw { amount: 0 }.data()) {
        return Withdraw::try_from_slice(payload)
            .map(|ix| HistoryEntryKind::Withdraw { amount: ix.amount })
            .unwrap_or(HistoryEntryKind::Unknown);
    }
    if disc
        == disc_of(
            AddBeneficiary {
                name: String::new(),
                address: Default::default(),
            }
            .data(),
        )
    {
        return AddBeneficiary::try_from_slice(payload)
            .map(|ix| HistoryEntryKind::AddBeneficiary { name: ix.name })
            .unwrap_or(HistoryEntryKind::Unknown);
    }
    if disc
        == disc_of(
            AddBeneficiaryUrgent {
                name: String::new(),
                address: Default::default(),
            }
            .data(),
        )
    {
        return AddBeneficiaryUrgent::try_from_slice(payload)
            .map(|ix| HistoryEntryKind::AddBeneficiaryUrgent { name: ix.name })
            .unwrap_or(HistoryEntryKind::Unknown);
    }
    if disc
        == disc_of(
            RequestWithdrawal {
                amount: 0,
                nonce: 0,
            }
            .data(),
        )
    {
        return RequestWithdrawal::try_from_slice(payload)
            .map(|ix| HistoryEntryKind::RequestWithdrawal {
                amount: ix.amount,
                nonce: ix.nonce,
            })
            .unwrap_or(HistoryEntryKind::Unknown);
    }
    if disc == disc_of(ApproveWithdrawal {}.data()) {
        return HistoryEntryKind::ApproveWithdrawal;
    }

    HistoryEntryKind::Unknown
}

/// Fetches up to `limit` recent signatures touching `vault`, decodes each
/// transaction's instructions that belong to `program_id`, and returns
/// the recognized ones as plain `HistoryEntry` values (an `Unknown`-
/// decoded instruction, or one from a different program in the same
/// transaction, is silently skipped — it is not history-worthy on its
/// own). A failed transaction (one that errored on-chain) never changed
/// vault state and is skipped entirely.
pub fn fetch_history(
    rpc: &RpcClient,
    program_id: &Pubkey,
    vault: &Pubkey,
    limit: usize,
) -> Result<Vec<HistoryEntry>, ActionError> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let signatures = rpc
        .get_signatures_for_address_with_config(
            vault,
            GetConfirmedSignaturesForAddress2Config {
                before: None,
                until: None,
                limit: Some(limit),
                commitment: None,
            },
        )
        .map_err(map_client_error)?;

    let program_id_str = program_id.to_string();
    let mut entries = Vec::new();
    for sig_info in signatures {
        if sig_info.err.is_some() {
            continue;
        }
        let Ok(signature) = sig_info.signature.parse::<solana_signature::Signature>() else {
            continue; // the node returned a signature we cannot parse; skip this entry
        };

        let tx = match rpc.get_transaction_with_config(
            &signature,
            RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Json),
                commitment: None,
                max_supported_transaction_version: Some(0),
            },
        ) {
            Ok(tx) => tx,
            Err(e) => match map_client_error(e) {
                // A connection-level failure means the whole history is unreliable.
                ActionError::NoConnection => return Err(ActionError::NoConnection),
                // One unavailable transaction (pruned, not found, odd encoding)
                // must not hide the rest of the history.
                // (`map_client_error` never yields `Unconfirmed`.)
                ActionError::Rejected(_) | ActionError::Unconfirmed { .. } => continue,
            },
        };

        let EncodedTransaction::Json(ui_tx) = tx.transaction.transaction else {
            continue;
        };
        let UiMessage::Raw(message) = ui_tx.message else {
            continue; // requested Json (not JsonParsed) encoding, so this is always Raw
        };

        let Some(program_index) = message
            .account_keys
            .iter()
            .position(|k| *k == program_id_str)
        else {
            continue; // this transaction never touched our program
        };

        for ix in &message.instructions {
            if ix.program_id_index as usize != program_index {
                continue; // a different program's instruction in the same tx
            }
            let Ok(raw) = bs58::decode(&ix.data).into_vec() else {
                continue;
            };
            let kind = decode_instruction_kind(&raw);
            if kind == HistoryEntryKind::Unknown {
                continue;
            }
            entries.push(HistoryEntry {
                signature: sig_info.signature.clone(),
                block_time: sig_info.block_time,
                kind,
            });
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::InstructionData;
    use family_vault::instruction::{
        AddBeneficiary, AddBeneficiaryUrgent, AddTrustedApprover, ApproveWithdrawal, Deposit,
        InitializeApprovers, InitializeVault, RequestWithdrawal, Withdraw,
    };

    #[test]
    fn decodes_initialize_vault() {
        let data = InitializeVault {
            is_pool: true,
            daily_limit: 1_000,
        }
        .data();
        assert_eq!(
            decode_instruction_kind(&data),
            HistoryEntryKind::InitializeVault
        );
    }

    #[test]
    fn decodes_initialize_approvers() {
        let data = InitializeApprovers {}.data();
        assert_eq!(
            decode_instruction_kind(&data),
            HistoryEntryKind::InitializeApprovers
        );
    }

    #[test]
    fn decodes_add_trusted_approver() {
        let data = AddTrustedApprover {
            address: Default::default(),
        }
        .data();
        assert_eq!(
            decode_instruction_kind(&data),
            HistoryEntryKind::AddTrustedApprover
        );
    }

    #[test]
    fn decodes_deposit_with_its_amount() {
        let data = Deposit { amount: 42_000 }.data();
        assert_eq!(
            decode_instruction_kind(&data),
            HistoryEntryKind::Deposit { amount: 42_000 }
        );
    }

    #[test]
    fn decodes_withdraw_with_its_amount() {
        let data = Withdraw { amount: 7_500 }.data();
        assert_eq!(
            decode_instruction_kind(&data),
            HistoryEntryKind::Withdraw { amount: 7_500 }
        );
    }

    #[test]
    fn decodes_add_beneficiary_with_its_name() {
        let data = AddBeneficiary {
            name: "Внук Саша".to_string(),
            address: Default::default(),
        }
        .data();
        assert_eq!(
            decode_instruction_kind(&data),
            HistoryEntryKind::AddBeneficiary {
                name: "Внук Саша".to_string()
            }
        );
    }

    #[test]
    fn decodes_add_beneficiary_urgent_with_its_name() {
        let data = AddBeneficiaryUrgent {
            name: "Дочка Маша".to_string(),
            address: Default::default(),
        }
        .data();
        assert_eq!(
            decode_instruction_kind(&data),
            HistoryEntryKind::AddBeneficiaryUrgent {
                name: "Дочка Маша".to_string()
            }
        );
    }

    #[test]
    fn decodes_request_withdrawal_with_amount_and_nonce() {
        let data = RequestWithdrawal {
            amount: 100,
            nonce: 3,
        }
        .data();
        assert_eq!(
            decode_instruction_kind(&data),
            HistoryEntryKind::RequestWithdrawal {
                amount: 100,
                nonce: 3
            }
        );
    }

    #[test]
    fn decodes_approve_withdrawal() {
        let data = ApproveWithdrawal {}.data();
        assert_eq!(
            decode_instruction_kind(&data),
            HistoryEntryKind::ApproveWithdrawal
        );
    }

    #[test]
    fn an_unrecognized_discriminator_decodes_as_unknown() {
        let garbage = vec![0xFFu8; 16];
        assert_eq!(decode_instruction_kind(&garbage), HistoryEntryKind::Unknown);
    }

    #[test]
    fn data_shorter_than_a_discriminator_decodes_as_unknown_not_a_panic() {
        assert_eq!(
            decode_instruction_kind(&[1, 2, 3]),
            HistoryEntryKind::Unknown
        );
        assert_eq!(decode_instruction_kind(&[]), HistoryEntryKind::Unknown);
    }

    #[test]
    fn a_known_discriminator_with_truncated_payload_decodes_as_unknown_not_a_panic() {
        // Deposit's discriminator with the u64 amount cut short.
        let mut data = Deposit { amount: 1 }.data();
        data.truncate(10); // 8-byte discriminator + 2 of the 8 amount bytes
        assert_eq!(decode_instruction_kind(&data), HistoryEntryKind::Unknown);
    }

    // Requires a live RPC endpoint and a vault with real transaction
    // history. Run manually with
    // `HISTORY_TEST_VAULT=<pubkey> cargo test -p family-app-core history::tests::fetches_real_history -- --ignored --nocapture`
    #[test]
    #[ignore = "needs a live RPC endpoint and a vault with real history"]
    fn fetches_real_history() {
        let rpc =
            solana_client::rpc_client::RpcClient::new("https://api.devnet.solana.com".to_string());
        let program_id: solana_pubkey::Pubkey = "9ozRBpTmDJJzcmJhKEbfvsoe7qqo2584bus1rSMG4zwJ"
            .parse()
            .unwrap();
        let vault: solana_pubkey::Pubkey = std::env::var("HISTORY_TEST_VAULT")
            .expect("set HISTORY_TEST_VAULT to a vault pubkey with real transaction history")
            .parse()
            .unwrap();
        let entries = fetch_history(&rpc, &program_id, &vault, 20).unwrap();
        println!("found {} entries: {entries:?}", entries.len());
        assert!(!entries.is_empty());
    }

    #[test]
    fn a_bare_discriminator_with_no_payload_is_unknown_when_a_payload_is_required() {
        let full = Deposit { amount: 1 }.data();
        assert_eq!(
            decode_instruction_kind(&full[..8]),
            HistoryEntryKind::Unknown
        );
    }

    #[test]
    fn seven_bytes_is_unknown() {
        let full = Deposit { amount: 1 }.data();
        assert_eq!(
            decode_instruction_kind(&full[..7]),
            HistoryEntryKind::Unknown
        );
    }

    #[test]
    fn trailing_bytes_after_a_valid_payload_are_unknown() {
        let mut data = Deposit { amount: 1 }.data();
        data.push(0);
        assert_eq!(decode_instruction_kind(&data), HistoryEntryKind::Unknown);
    }

    #[test]
    fn a_huge_string_length_prefix_does_not_panic_or_allocate_it() {
        let mut data = AddBeneficiary {
            name: String::new(),
            address: Default::default(),
        }
        .data();
        // Overwrite the 4-byte length prefix right after the discriminator.
        data[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(decode_instruction_kind(&data), HistoryEntryKind::Unknown);
    }

    #[test]
    fn an_invalid_utf8_name_is_unknown() {
        let mut data = AddBeneficiary {
            name: "ab".to_string(),
            address: Default::default(),
        }
        .data();
        data[12] = 0xFF;
        data[13] = 0xFE;
        assert_eq!(decode_instruction_kind(&data), HistoryEntryKind::Unknown);
    }
}
