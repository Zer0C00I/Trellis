//! Finds every vault a pubkey participates in (as a payee or as a trusted
//! approver) by reading the `family-vault` program's list accounts directly
//! — no separate index or backend.
//!
//! Why two requests and a client-side check, not one filtered request per
//! slot: `getProgramAccounts` filters are ANDed, so "is my key in any of the
//! 10 payee slots" cannot be one filtered call, and 15 filtered calls (one
//! per slot) exceed the public Devnet node's limit of 10 `getProgramAccounts`
//! per 10 seconds, which turns a launch into a 10+ second wait on HTTP 429
//! retries. Asking for every list of one kind (filtered by exact account
//! size only) takes 2 requests and moves the "is it me" test to the client.
//! That is the right trade at MVP scale (each list is 851 or 201 bytes, so
//! even a thousand vaults is about a megabyte); a large deployment should put
//! an indexer or a provider-side filter in front of this.

use std::collections::BTreeSet;

use anchor_lang::AccountDeserialize;
use family_vault::state::{BeneficiaryList, TrustedApprovers, MAX_APPROVERS, MAX_BENEFICIARIES};
use solana_account_decoder_client_types::UiAccountEncoding;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::RpcFilterType;
use solana_pubkey::Pubkey;

use crate::error::{map_client_error, ActionError};

/// The raw data of every program account of exactly `data_size` bytes.
fn fetch_accounts_of_size(
    rpc: &RpcClient,
    program_id: &Pubkey,
    data_size: u64,
) -> Result<Vec<Vec<u8>>, ActionError> {
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![RpcFilterType::DataSize(data_size)]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: None,
            commitment: None,
            min_context_slot: None,
        },
        with_context: Some(false),
        sort_results: Some(false),
    };

    rpc.get_program_ui_accounts_with_config(program_id, config)
        .map_err(map_client_error)?
        .into_iter()
        .map(|(_pubkey, ui_account)| {
            ui_account
                .to_account()
                .map(|account| account.data)
                .ok_or_else(|| ActionError::Rejected("could not decode account data".to_string()))
        })
        .collect()
}

/// Only the first `count` entries are live; the rest of the fixed array is
/// unused space. `count` comes from chain data, so it is clamped.
fn live_entries<T>(entries: &[T], count: u8) -> &[T] {
    &entries[..(count as usize).min(entries.len())]
}

/// Whether `me` is one of the payees of this list.
fn payee_list_has(list: &BeneficiaryList, me: &Pubkey) -> bool {
    let me = family_vault_client::to_anchor(*me);
    live_entries(&list.entries[..MAX_BENEFICIARIES], list.count)
        .iter()
        .any(|entry| entry.address == me)
}

/// Whether `me` is one of the trusted approvers of this list.
fn approver_list_has(list: &TrustedApprovers, me: &Pubkey) -> bool {
    let me = family_vault_client::to_anchor(*me);
    live_entries(&list.entries[..MAX_APPROVERS], list.count).contains(&me)
}

/// Every vault `me` participates in, as payee or approver or both,
/// deduplicated and sorted. Two requests. Any failure fails the whole call:
/// a vault must never be silently missing. MVP scope decision (family-app
/// design spec): the UI shows one vault at a time, but this returns them all.
pub fn discover_my_vaults(
    rpc: &RpcClient,
    program_id: &Pubkey,
    me: &Pubkey,
) -> Result<Vec<Pubkey>, ActionError> {
    let mut vaults = BTreeSet::new();
    for data in fetch_accounts_of_size(rpc, program_id, BeneficiaryList::SPACE as u64)? {
        let list = BeneficiaryList::try_deserialize(&mut &data[..])
            .map_err(|e| ActionError::Rejected(format!("decoding BeneficiaryList: {e}")))?;
        if payee_list_has(&list, me) {
            vaults.insert(family_vault_client::to_client(list.vault));
        }
    }
    for data in fetch_accounts_of_size(rpc, program_id, TrustedApprovers::SPACE as u64)? {
        let list = TrustedApprovers::try_deserialize(&mut &data[..])
            .map_err(|e| ActionError::Rejected(format!("decoding TrustedApprovers: {e}")))?;
        if approver_list_has(&list, me) {
            vaults.insert(family_vault_client::to_client(list.vault));
        }
    }
    Ok(vaults.into_iter().collect())
}

/// The two vault addresses a key can own: its personal vault and its pool
/// vault. Being the owner puts a key in no beneficiary or approver list, so
/// [`discover_my_vaults`] cannot find these.
pub fn owned_vault_addresses(program_id: &Pubkey, owner: &Pubkey) -> [Pubkey; 2] {
    [
        family_vault_client::vault_pda(program_id, owner, true).0,
        family_vault_client::vault_pda(program_id, owner, false).0,
    ]
}

/// The vaults `owner` has already created (a pool and/or a personal vault).
pub fn discover_owned_vaults(
    rpc: &RpcClient,
    program_id: &Pubkey,
    owner: &Pubkey,
) -> Result<Vec<Pubkey>, ActionError> {
    let candidates = owned_vault_addresses(program_id, owner);
    let accounts = rpc
        .get_multiple_accounts(&candidates)
        .map_err(crate::error::map_client_error)?;
    if accounts.len() != candidates.len() {
        return Err(ActionError::Rejected(
            "the node returned an unexpected number of accounts".to_string(),
        ));
    }
    Ok(candidates
        .into_iter()
        .zip(accounts)
        .filter_map(|(address, account)| match account {
            // A PDA of this program that exists is a vault; anything else at
            // that address (nothing, or another owner) is not ours.
            Some(account) if account.owner == *program_id => Some(address),
            _ => None,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::AccountSerialize;
    use family_vault::state::{Beneficiary, BENEFICIARY_NAME_LEN};

    fn anchor_key(byte: u8) -> anchor_lang::prelude::Pubkey {
        anchor_lang::prelude::Pubkey::new_from_array([byte; 32])
    }

    fn client_key(byte: u8) -> Pubkey {
        Pubkey::new_from_array([byte; 32])
    }

    fn payee(byte: u8) -> Beneficiary {
        Beneficiary {
            address: anchor_key(byte),
            name: [0u8; BENEFICIARY_NAME_LEN],
            name_len: 0,
            added_at: 0,
            active_after: 0,
        }
    }

    fn list_with(count: u8, bytes: &[u8]) -> BeneficiaryList {
        let mut entries = [payee(0); MAX_BENEFICIARIES];
        for (slot, byte) in bytes.iter().enumerate() {
            entries[slot] = payee(*byte);
        }
        BeneficiaryList {
            vault: anchor_key(99),
            count,
            entries,
        }
    }

    #[test]
    fn a_payee_is_found_in_any_live_slot() {
        let list = list_with(3, &[1, 2, 3]);
        assert!(payee_list_has(&list, &client_key(1)));
        assert!(payee_list_has(&list, &client_key(3)));
        assert!(!payee_list_has(&list, &client_key(4)));
    }

    #[test]
    fn an_entry_past_count_does_not_count() {
        // Slot 2 holds key 3 but only two entries are live.
        let list = list_with(2, &[1, 2, 3]);
        assert!(!payee_list_has(&list, &client_key(3)));
    }

    #[test]
    fn a_garbage_count_cannot_read_out_of_range() {
        let list = list_with(255, &[1]);
        assert!(payee_list_has(&list, &client_key(1)));
        assert!(!payee_list_has(&list, &client_key(7)));
    }

    #[test]
    fn approvers_are_found_the_same_way() {
        let mut entries = [anchor_key(0); MAX_APPROVERS];
        entries[0] = anchor_key(5);
        entries[1] = anchor_key(6);
        let list = TrustedApprovers {
            vault: anchor_key(99),
            count: 2,
            entries,
        };
        assert!(approver_list_has(&list, &client_key(5)));
        assert!(approver_list_has(&list, &client_key(6)));
        assert!(!approver_list_has(&list, &client_key(7)));
        let none = TrustedApprovers { count: 0, ..list };
        assert!(!approver_list_has(&none, &client_key(5)));
    }

    #[test]
    fn real_serialized_lists_have_the_sizes_the_scan_asks_for() {
        let mut bytes = Vec::new();
        list_with(1, &[1]).try_serialize(&mut bytes).unwrap();
        assert_eq!(bytes.len(), BeneficiaryList::SPACE);
        let list = BeneficiaryList::try_deserialize(&mut &bytes[..]).unwrap();
        assert!(payee_list_has(&list, &client_key(1)));

        let approvers = TrustedApprovers {
            vault: anchor_key(99),
            count: 1,
            entries: [anchor_key(5); MAX_APPROVERS],
        };
        let mut bytes = Vec::new();
        approvers.try_serialize(&mut bytes).unwrap();
        assert_eq!(bytes.len(), TrustedApprovers::SPACE);
    }

    #[test]
    fn owned_vault_addresses_are_the_pool_and_personal_pdas() {
        let program_id = Pubkey::new_from_array([1u8; 32]);
        let owner = Pubkey::new_from_array([2u8; 32]);
        let [pool, personal] = owned_vault_addresses(&program_id, &owner);
        assert_ne!(pool, personal);
        assert_eq!(
            pool,
            family_vault_client::vault_pda(&program_id, &owner, true).0
        );
        assert_eq!(
            personal,
            family_vault_client::vault_pda(&program_id, &owner, false).0
        );
        let other = Pubkey::new_from_array([3u8; 32]);
        assert_ne!(pool, owned_vault_addresses(&program_id, &other)[0]);
    }

    // Requires a live RPC endpoint with real BeneficiaryList/TrustedApprovers
    // accounts to scan. Run manually with
    // `cargo test -p family-app-core discovery::tests::discovers_a_real_vault -- --ignored --nocapture`
    // with DISCOVERY_TEST_PUBKEY set to a pubkey known (from devnet) to
    // actually appear in at least one vault's beneficiary or approver list.
    #[test]
    #[ignore = "needs a live RPC endpoint and a known devnet vault membership"]
    fn discovers_a_real_vault() {
        let rpc =
            solana_client::rpc_client::RpcClient::new("https://api.devnet.solana.com".to_string());
        let program_id: solana_pubkey::Pubkey = "9ozRBpTmDJJzcmJhKEbfvsoe7qqo2584bus1rSMG4zwJ"
            .parse()
            .unwrap();
        let me: solana_pubkey::Pubkey = std::env::var("DISCOVERY_TEST_PUBKEY")
            .expect("set DISCOVERY_TEST_PUBKEY to a pubkey known to be a beneficiary or approver")
            .parse()
            .unwrap();
        let vaults = discover_my_vaults(&rpc, &program_id, &me).unwrap();
        println!("found {} vault(s): {vaults:?}", vaults.len());
        assert!(!vaults.is_empty());
    }
}
