//! Read-only transaction history for a vault: what happened, in plain
//! terms, without exposing raw instruction bytes to a caller. Two RPC
//! calls per signature (`getSignaturesForAddress` then `getTransaction`)
//! — a completely different Solana RPC surface than `discovery`'s
//! `getProgramAccounts` scan, since history lives in the ledger, not in
//! any account's current state.
//!
//! A confirmed transaction never changes, so [`fetch_history_cached`] keeps
//! the decoded entries of each signature in a caller-owned, bounded
//! [`HistoryCache`] and only calls `getTransaction` for signatures it has
//! not seen; `getSignaturesForAddress` still runs every time. Transactions
//! that could not be fetched are never cached.

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
use std::collections::{HashMap, VecDeque};

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

/// Decoded transactions by signature, owned by the caller and passed to
/// [`fetch_history_cached`]. A confirmed transaction never changes, so once
/// its entries are known (possibly none: a foreign or unrecognized
/// transaction) it is never fetched again. Bounded: when over capacity the
/// oldest-inserted signatures are evicted first. Give it a capacity of at
/// least the `limit` you fetch with, or every call evicts what it needs next
/// and the cache thrashes.
pub struct HistoryCache {
    capacity: usize,
    entries: HashMap<String, Vec<HistoryEntry>>,
    order: VecDeque<String>,
}

impl HistoryCache {
    /// A cache holding at most `capacity` transactions (0 caches nothing).
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn get(&self, signature: &str) -> Option<&Vec<HistoryEntry>> {
        self.entries.get(signature)
    }

    /// A transaction first seen with a null block time may get one later:
    /// keep the newest known time (a null never erases a known one).
    fn refresh_block_time(&mut self, signature: &str, block_time: Option<i64>) {
        if let (Some(time), Some(entries)) = (block_time, self.entries.get_mut(signature)) {
            for entry in entries {
                entry.block_time = Some(time);
            }
        }
    }

    fn insert(&mut self, signature: String, entries: Vec<HistoryEntry>) {
        if self.entries.insert(signature.clone(), entries).is_none() {
            self.order.push_back(signature);
        }
        while self.entries.len() > self.capacity {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            self.entries.remove(&oldest);
        }
    }
}

/// Fetches up to `limit` recent signatures touching `vault`, decodes each
/// transaction's instructions that belong to `program_id`, and returns
/// the recognized ones as plain `HistoryEntry` values (an `Unknown`-
/// decoded instruction, or one from a different program in the same
/// transaction, is silently skipped — it is not history-worthy on its
/// own). A failed transaction (one that errored on-chain) never changed
/// vault state and is skipped entirely. Every call refetches every
/// transaction; use [`fetch_history_cached`] to avoid that on refresh.
pub fn fetch_history(
    rpc: &RpcClient,
    program_id: &Pubkey,
    vault: &Pubkey,
    limit: usize,
) -> Result<Vec<HistoryEntry>, ActionError> {
    fetch_history_cached(rpc, program_id, vault, limit, &mut HistoryCache::new(0))
}

/// Same as [`fetch_history`], but `getTransaction` is only called for
/// signatures not already in `cache`. The signature list is still fetched
/// every time (new transactions appear), and the result is identical to an
/// uncached fetch. A transaction that could not be fetched (pruned, not
/// found, odd encoding) is not cached, since it may succeed later.
pub fn fetch_history_cached(
    rpc: &RpcClient,
    program_id: &Pubkey,
    vault: &Pubkey,
    limit: usize,
    cache: &mut HistoryCache,
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
    let signatures: Vec<(String, Option<i64>)> = signatures
        .into_iter()
        .filter(|sig_info| sig_info.err.is_none())
        .map(|sig_info| (sig_info.signature, sig_info.block_time))
        .collect();

    collect_history(&signatures, cache, |signature, block_time| {
        let Ok(parsed) = signature.parse::<solana_signature::Signature>() else {
            return Ok(None); // the node returned a signature we cannot parse; skip this entry
        };
        let tx = match rpc.get_transaction_with_config(
            &parsed,
            RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Json),
                commitment: None,
                max_supported_transaction_version: Some(0),
            },
        ) {
            Ok(tx) => tx,
            Err(e) => {
                return match map_client_error(e) {
                    // A connection-level failure means the whole history is unreliable.
                    ActionError::NoConnection => Err(ActionError::NoConnection),
                    // One unavailable transaction (pruned, not found, odd encoding)
                    // must not hide the rest of the history.
                    // (`map_client_error` never yields `Unconfirmed`.)
                    ActionError::Rejected(_) | ActionError::Unconfirmed { .. } => Ok(None),
                };
            }
        };
        Ok(decode_transaction(
            &tx.transaction.transaction,
            &program_id_str,
            signature,
            block_time,
        ))
    })
}

/// The cache logic, independent of the network. `signatures` are the
/// successful ones, newest first. `fetch` returns `Ok(Some(entries))` for a
/// definitive result (cached, even if empty), `Ok(None)` for "could not get
/// this transaction" (skipped, not cached), `Err` to abort the whole call.
fn collect_history(
    signatures: &[(String, Option<i64>)],
    cache: &mut HistoryCache,
    mut fetch: impl FnMut(&str, Option<i64>) -> Result<Option<Vec<HistoryEntry>>, ActionError>,
) -> Result<Vec<HistoryEntry>, ActionError> {
    let mut entries = Vec::new();
    for (signature, block_time) in signatures {
        cache.refresh_block_time(signature, *block_time);
        if let Some(cached) = cache.get(signature) {
            entries.extend(cached.iter().cloned());
            continue;
        }
        if let Some(decoded) = fetch(signature, *block_time)? {
            entries.extend(decoded.iter().cloned());
            cache.insert(signature.clone(), decoded);
        }
    }
    Ok(entries)
}

/// The recognized instructions of one fetched transaction that belong to
/// the program. `None` when the transaction came back in a shape we did not
/// request (not a definitive answer, so not cached); `Some(vec![])` when it
/// simply has nothing for us.
fn decode_transaction(
    transaction: &EncodedTransaction,
    program_id_str: &str,
    signature: &str,
    block_time: Option<i64>,
) -> Option<Vec<HistoryEntry>> {
    let EncodedTransaction::Json(ui_tx) = transaction else {
        return None;
    };
    let UiMessage::Raw(message) = &ui_tx.message else {
        return None; // requested Json (not JsonParsed) encoding, so this is always Raw
    };

    let Some(program_index) = message
        .account_keys
        .iter()
        .position(|k| k == program_id_str)
    else {
        return Some(Vec::new()); // this transaction never touched our program
    };

    let mut entries = Vec::new();
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
            signature: signature.to_string(),
            block_time,
            kind,
        });
    }
    Some(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::InstructionData;
    use family_vault::instruction::{
        AddBeneficiary, AddBeneficiaryUrgent, AddTrustedApprover, ApproveWithdrawal, Deposit,
        InitializeApprovers, InitializeVault, RequestWithdrawal, Withdraw,
    };

    fn entry(signature: &str) -> HistoryEntry {
        HistoryEntry {
            signature: signature.to_string(),
            block_time: Some(1),
            kind: HistoryEntryKind::ApproveWithdrawal,
        }
    }

    #[test]
    fn a_cache_hit_takes_the_block_time_from_the_current_listing() {
        let mut cache = HistoryCache::new(10);
        let mut fetch = |sig: &str, _t: Option<i64>| Ok(Some(vec![entry(sig)]));
        let first = vec![("a".to_string(), None)];
        collect_history(&first, &mut cache, &mut fetch).unwrap();
        let later = vec![("a".to_string(), Some(555))];
        let got = collect_history(&later, &mut cache, &mut fetch).unwrap();
        assert_eq!(got[0].block_time, Some(555));
        // and a later null does not erase it
        let again = collect_history(&first, &mut cache, &mut fetch).unwrap();
        assert_eq!(again[0].block_time, Some(555));
    }

    fn sigs(names: &[&str]) -> Vec<(String, Option<i64>)> {
        names.iter().map(|n| (n.to_string(), Some(1))).collect()
    }

    #[test]
    fn a_cached_signature_is_not_fetched_again() {
        let mut cache = HistoryCache::new(10);
        let mut calls = 0;
        let mut fetch = |sig: &str, _: Option<i64>| {
            calls += 1;
            Ok(Some(vec![entry(sig)]))
        };
        let first = collect_history(&sigs(&["a", "b"]), &mut cache, &mut fetch).unwrap();
        let second = collect_history(&sigs(&["a", "b"]), &mut cache, &mut fetch).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 2);
        assert_eq!(calls, 2);
    }

    #[test]
    fn only_unseen_signatures_are_fetched_and_order_is_kept() {
        let mut cache = HistoryCache::new(10);
        let mut fetched = Vec::new();
        let mut fetch = |sig: &str, _: Option<i64>| {
            fetched.push(sig.to_string());
            Ok(Some(vec![entry(sig)]))
        };
        collect_history(&sigs(&["a"]), &mut cache, &mut fetch).unwrap();
        let got = collect_history(&sigs(&["n", "a"]), &mut cache, &mut fetch).unwrap();
        assert_eq!(fetched, ["a", "n"]);
        let order: Vec<_> = got.iter().map(|e| e.signature.as_str()).collect();
        assert_eq!(order, ["n", "a"]);
    }

    #[test]
    fn a_definitive_empty_result_is_cached() {
        let mut cache = HistoryCache::new(10);
        let mut calls = 0;
        let mut fetch = |_: &str, _: Option<i64>| {
            calls += 1;
            Ok(Some(Vec::new()))
        };
        assert!(collect_history(&sigs(&["a"]), &mut cache, &mut fetch)
            .unwrap()
            .is_empty());
        collect_history(&sigs(&["a"]), &mut cache, &mut fetch).unwrap();
        assert_eq!(calls, 1);
    }

    #[test]
    fn a_failed_fetch_is_not_cached() {
        let mut cache = HistoryCache::new(10);
        let mut calls = 0;
        let mut fetch = |sig: &str, _: Option<i64>| {
            calls += 1;
            Ok(if calls == 1 {
                None
            } else {
                Some(vec![entry(sig)])
            })
        };
        assert!(collect_history(&sigs(&["a"]), &mut cache, &mut fetch)
            .unwrap()
            .is_empty());
        let retried = collect_history(&sigs(&["a"]), &mut cache, &mut fetch).unwrap();
        assert_eq!(retried.len(), 1);
        assert_eq!(calls, 2);
    }

    #[test]
    fn an_aborting_error_is_returned_and_nothing_is_cached_for_it() {
        let mut cache = HistoryCache::new(10);
        let mut fetch = |_: &str, _: Option<i64>| Err(ActionError::NoConnection);
        let got = collect_history(&sigs(&["a"]), &mut cache, &mut fetch);
        assert!(matches!(got, Err(ActionError::NoConnection)));
        assert!(cache.get("a").is_none());
    }

    #[test]
    fn the_oldest_inserted_entry_is_evicted_at_capacity() {
        let mut cache = HistoryCache::new(2);
        let mut fetched = Vec::new();
        let mut fetch = |sig: &str, _: Option<i64>| {
            fetched.push(sig.to_string());
            Ok(Some(vec![entry(sig)]))
        };
        collect_history(&sigs(&["a", "b", "c"]), &mut cache, &mut fetch).unwrap();
        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_some() && cache.get("c").is_some());
        collect_history(&sigs(&["a"]), &mut cache, &mut fetch).unwrap();
        assert_eq!(fetched, ["a", "b", "c", "a"]);
    }

    #[test]
    fn a_zero_capacity_cache_stores_nothing() {
        let mut cache = HistoryCache::new(0);
        let mut calls = 0;
        let mut fetch = |sig: &str, _: Option<i64>| {
            calls += 1;
            Ok(Some(vec![entry(sig)]))
        };
        collect_history(&sigs(&["a"]), &mut cache, &mut fetch).unwrap();
        collect_history(&sigs(&["a"]), &mut cache, &mut fetch).unwrap();
        assert_eq!(calls, 2);
    }

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
