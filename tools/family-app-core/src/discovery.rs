//! Finds every vault a pubkey participates in (payee or trusted approver) by reading the program's list
//! accounts directly, with no index or backend.
//!
//! Two requests plus a client-side check, not one filtered request per slot: `getProgramAccounts` filters are
//! ANDed, so "my key in any of 10 slots" cannot be one call, and 15 per-slot calls exceed the public Devnet
//! limit of 10 per 10 seconds (10+ seconds of HTTP 429 retries). Fetching every list of one kind, filtered by
//! exact size and discriminator, takes 2 requests. That is fine at MVP scale (lists are 851 or 201 bytes, so a
//! thousand vaults is about a megabyte); a larger deployment needs an indexer.

use std::collections::BTreeSet;

use anchor_lang::{AccountDeserialize, Discriminator};
use family_vault::state::{BeneficiaryList, TrustedApprovers, MAX_APPROVERS, MAX_BENEFICIARIES};
use solana_account_decoder_client_types::UiAccountEncoding;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_pubkey::Pubkey;

use crate::error::{map_client_error, ActionError};

/// Filters for one list kind: exactly `data_size` bytes AND the account `discriminator` (filters are ANDed).
fn list_filters(data_size: u64, discriminator: &[u8]) -> Vec<RpcFilterType> {
    vec![
        RpcFilterType::DataSize(data_size),
        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(0, discriminator)),
    ]
}

/// The raw data of every program account of exactly `data_size` bytes whose
/// first 8 bytes are `discriminator`.
fn fetch_accounts_of_kind(
    rpc: &RpcClient,
    program_id: &Pubkey,
    data_size: u64,
    discriminator: &[u8],
) -> Result<Vec<Vec<u8>>, ActionError> {
    let config = RpcProgramAccountsConfig {
        filters: Some(list_filters(data_size, discriminator)),
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
/// a vault must never be silently missing.
pub fn discover_my_vaults(
    rpc: &RpcClient,
    program_id: &Pubkey,
    me: &Pubkey,
) -> Result<Vec<Pubkey>, ActionError> {
    let mut vaults = BTreeSet::new();
    for data in fetch_accounts_of_kind(
        rpc,
        program_id,
        BeneficiaryList::SPACE as u64,
        BeneficiaryList::DISCRIMINATOR,
    )? {
        let list = BeneficiaryList::try_deserialize(&mut &data[..])
            .map_err(|e| ActionError::Rejected(format!("decoding BeneficiaryList: {e}")))?;
        if payee_list_has(&list, me) {
            vaults.insert(family_vault_client::to_client(list.vault));
        }
    }
    for data in fetch_accounts_of_kind(
        rpc,
        program_id,
        TrustedApprovers::SPACE as u64,
        TrustedApprovers::DISCRIMINATOR,
    )? {
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
    owned_vaults_from(
        candidates,
        accounts
            .into_iter()
            .map(|account| account.map(|account| account.owner))
            .collect(),
        program_id,
    )
}

/// Which of the two `candidates` are vaults: those whose account exists and is owned by `program_id`.
/// `owners[i]` is the owner of `candidates[i]`, or `None` if missing. Order is kept.
fn owned_vaults_from(
    candidates: [Pubkey; 2],
    owners: Vec<Option<Pubkey>>,
    program_id: &Pubkey,
) -> Result<Vec<Pubkey>, ActionError> {
    if owners.len() != candidates.len() {
        return Err(ActionError::Rejected(
            "the node returned an unexpected number of accounts".to_string(),
        ));
    }
    Ok(candidates
        .into_iter()
        .zip(owners)
        .filter_map(|(address, owner)| match owner {
            // A PDA of this program that exists is a vault; anything else at
            // that address (nothing, or another owner) is not ours.
            Some(owner) if owner == *program_id => Some(address),
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
    fn the_discriminator_filters_use_real_distinct_eight_byte_prefixes() {
        assert_eq!(BeneficiaryList::DISCRIMINATOR.len(), 8);
        assert_eq!(TrustedApprovers::DISCRIMINATOR.len(), 8);
        assert_ne!(
            BeneficiaryList::DISCRIMINATOR,
            TrustedApprovers::DISCRIMINATOR
        );

        let mut bytes = Vec::new();
        list_with(1, &[1]).try_serialize(&mut bytes).unwrap();
        assert_eq!(&bytes[..8], BeneficiaryList::DISCRIMINATOR);

        let approvers = TrustedApprovers {
            vault: anchor_key(99),
            count: 1,
            entries: [anchor_key(5); MAX_APPROVERS],
        };
        let mut bytes = Vec::new();
        approvers.try_serialize(&mut bytes).unwrap();
        assert_eq!(&bytes[..8], TrustedApprovers::DISCRIMINATOR);
    }

    #[test]
    fn a_list_request_filters_by_size_and_discriminator() {
        let filters = list_filters(851, BeneficiaryList::DISCRIMINATOR);
        assert_eq!(filters.len(), 2);
        assert!(matches!(filters[0], RpcFilterType::DataSize(851)));
        assert_eq!(
            filters[1],
            RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
                0,
                BeneficiaryList::DISCRIMINATOR
            ))
        );
    }

    fn two_candidates() -> [Pubkey; 2] {
        [client_key(10), client_key(11)]
    }

    #[test]
    fn owned_vaults_include_only_accounts_owned_by_the_program() {
        let program = client_key(1);
        let other = client_key(2);
        let [a, b] = two_candidates();

        let both = owned_vaults_from(
            two_candidates(),
            vec![Some(program), Some(program)],
            &program,
        );
        assert_eq!(both.unwrap(), vec![a, b]);

        let foreign =
            owned_vaults_from(two_candidates(), vec![Some(other), Some(program)], &program);
        assert_eq!(foreign.unwrap(), vec![b]);

        let missing = owned_vaults_from(two_candidates(), vec![None, Some(program)], &program);
        assert_eq!(missing.unwrap(), vec![b]);

        let none = owned_vaults_from(two_candidates(), vec![None, Some(other)], &program);
        assert!(none.unwrap().is_empty());
    }

    #[test]
    fn owned_vaults_keep_the_candidate_order() {
        let program = client_key(1);
        let got = owned_vaults_from(
            [client_key(11), client_key(10)],
            vec![Some(program); 2],
            &program,
        );
        assert_eq!(got.unwrap(), vec![client_key(11), client_key(10)]);
    }

    #[test]
    fn a_wrong_number_of_accounts_is_rejected() {
        let program = client_key(1);
        for owners in [vec![], vec![Some(program)], vec![Some(program); 3]] {
            let got = owned_vaults_from(two_candidates(), owners, &program);
            assert!(matches!(got, Err(ActionError::Rejected(_))));
        }
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
