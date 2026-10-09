//! Finds every vault a pubkey participates in (as a payee or as a trusted
//! approver) by scanning `family-vault` program accounts directly — no
//! separate index or backend. See the family-app design spec's "Vault
//! Discovery" section for why this takes 15 RPC calls, not one.

use family_vault::state::{BENEFICIARY_NAME_LEN, MAX_APPROVERS, MAX_BENEFICIARIES};

const DISCRIMINATOR_LEN: usize = 8;
const VAULT_FIELD_LEN: usize = 32;
const COUNT_FIELD_LEN: usize = 1;
const HEADER_LEN: usize = DISCRIMINATOR_LEN + VAULT_FIELD_LEN + COUNT_FIELD_LEN;
const BENEFICIARY_ENTRY_LEN: usize = 32 + BENEFICIARY_NAME_LEN + 1 + 8 + 8;

fn beneficiary_address_offset(slot: usize) -> usize {
    HEADER_LEN + slot * BENEFICIARY_ENTRY_LEN
}

fn approver_offset(slot: usize) -> usize {
    HEADER_LEN + slot * 32
}

use std::collections::BTreeSet;

use crate::error::{map_client_error, ActionError};
use anchor_lang::AccountDeserialize;
use family_vault::state::{BeneficiaryList, TrustedApprovers};
use solana_account_decoder_client_types::UiAccountEncoding;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_pubkey::Pubkey;

fn scan_for_pubkey_at_offset(
    rpc: &RpcClient,
    program_id: &Pubkey,
    data_size: u64,
    offset: usize,
    needle: &Pubkey,
) -> Result<Vec<Vec<u8>>, ActionError> {
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![
            RpcFilterType::DataSize(data_size),
            // base58 rather than a raw byte array: every RPC provider accepts it.
            RpcFilterType::Memcmp(Memcmp::new_base58_encoded(offset, &needle.to_bytes())),
        ]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: None,
            commitment: None,
            min_context_slot: None,
        },
        with_context: Some(false),
        sort_results: Some(false),
    };

    let accounts = rpc
        .get_program_ui_accounts_with_config(program_id, config)
        .map_err(map_client_error)?;

    accounts
        .into_iter()
        .map(|(_pubkey, ui_account)| {
            ui_account
                .to_account()
                .map(|account| account.data)
                .ok_or_else(|| ActionError::Rejected("could not decode account data".to_string()))
        })
        .collect()
}

/// Every vault where `me` is currently a payee.
fn vaults_where_i_am_a_beneficiary(
    rpc: &RpcClient,
    program_id: &Pubkey,
    me: &Pubkey,
) -> Result<BTreeSet<Pubkey>, ActionError> {
    let mut vaults = BTreeSet::new();
    for slot in 0..MAX_BENEFICIARIES {
        let raw_accounts = scan_for_pubkey_at_offset(
            rpc,
            program_id,
            BeneficiaryList::SPACE as u64,
            beneficiary_address_offset(slot),
            me,
        )?;
        for data in raw_accounts {
            let list = BeneficiaryList::try_deserialize(&mut &data[..])
                .map_err(|e| ActionError::Rejected(format!("decoding BeneficiaryList: {e}")))?;
            vaults.insert(family_vault_client::to_client(list.vault));
        }
    }
    Ok(vaults)
}

/// Every vault where `me` is currently a trusted approver.
fn vaults_where_i_am_an_approver(
    rpc: &RpcClient,
    program_id: &Pubkey,
    me: &Pubkey,
) -> Result<BTreeSet<Pubkey>, ActionError> {
    let mut vaults = BTreeSet::new();
    for slot in 0..MAX_APPROVERS {
        let raw_accounts = scan_for_pubkey_at_offset(
            rpc,
            program_id,
            TrustedApprovers::SPACE as u64,
            approver_offset(slot),
            me,
        )?;
        for data in raw_accounts {
            let approvers = TrustedApprovers::try_deserialize(&mut &data[..])
                .map_err(|e| ActionError::Rejected(format!("decoding TrustedApprovers: {e}")))?;
            vaults.insert(family_vault_client::to_client(approvers.vault));
        }
    }
    Ok(vaults)
}

/// Every vault `me` participates in, as payee or approver or both,
/// deduplicated. MVP scope decision (family-app design spec): the UI only
/// shows the first one found; this function already returns the full set
/// so a future multi-vault switcher is a UI-only change.
pub fn discover_my_vaults(
    rpc: &RpcClient,
    program_id: &Pubkey,
    me: &Pubkey,
) -> Result<Vec<Pubkey>, ActionError> {
    let mut vaults = vaults_where_i_am_a_beneficiary(rpc, program_id, me)?;
    vaults.extend(vaults_where_i_am_an_approver(rpc, program_id, me)?);
    Ok(vaults.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use family_vault::state::{BENEFICIARY_NAME_LEN, MAX_APPROVERS, MAX_BENEFICIARIES};

    #[test]
    fn beneficiary_address_offset_is_after_the_discriminator_vault_and_count() {
        // 8 (discriminator) + 32 (vault) + 1 (count) = 41 for slot 0.
        assert_eq!(beneficiary_address_offset(0), 41);
    }

    #[test]
    fn beneficiary_address_offset_advances_by_one_full_entry_per_slot() {
        let entry_len = 32 + BENEFICIARY_NAME_LEN + 1 + 8 + 8; // address+name+name_len+added_at+active_after
        assert_eq!(
            beneficiary_address_offset(1) - beneficiary_address_offset(0),
            entry_len
        );
        assert_eq!(
            beneficiary_address_offset(MAX_BENEFICIARIES - 1) - beneficiary_address_offset(0),
            entry_len * (MAX_BENEFICIARIES - 1)
        );
    }

    #[test]
    fn approver_offset_is_after_the_discriminator_vault_and_count() {
        assert_eq!(approver_offset(0), 41);
    }

    #[test]
    fn approver_offset_advances_by_one_pubkey_per_slot() {
        assert_eq!(approver_offset(1) - approver_offset(0), 32);
        assert_eq!(
            approver_offset(MAX_APPROVERS - 1) - approver_offset(0),
            32 * (MAX_APPROVERS - 1)
        );
    }

    // The offsets must point at the right bytes of a real serialized account,
    // not just agree with a formula that mirrors the implementation.
    #[test]
    fn offsets_point_at_the_right_bytes_of_a_real_beneficiary_list() {
        use anchor_lang::AccountSerialize;
        use family_vault::state::{Beneficiary, BeneficiaryList};

        let empty = Beneficiary {
            address: anchor_lang::prelude::Pubkey::default(),
            name: [0u8; BENEFICIARY_NAME_LEN],
            name_len: 0,
            added_at: 0,
            active_after: 0,
        };
        let mut entries = [empty; MAX_BENEFICIARIES];
        for (slot, entry) in entries.iter_mut().enumerate() {
            entry.address = anchor_lang::prelude::Pubkey::new_from_array([slot as u8 + 1; 32]);
        }
        let list = BeneficiaryList {
            vault: anchor_lang::prelude::Pubkey::new_from_array([0xAA; 32]),
            count: MAX_BENEFICIARIES as u8,
            entries,
        };
        let mut bytes = Vec::new();
        list.try_serialize(&mut bytes).unwrap();

        assert_eq!(bytes.len(), BeneficiaryList::SPACE);
        assert_eq!(beneficiary_address_offset(0), 41);
        assert_eq!(beneficiary_address_offset(1), 122);
        assert_eq!(beneficiary_address_offset(9), 770);
        for slot in 0..MAX_BENEFICIARIES {
            let at = beneficiary_address_offset(slot);
            assert_eq!(&bytes[at..at + 32], &[slot as u8 + 1; 32], "slot {slot}");
        }
    }

    #[test]
    fn offsets_point_at_the_right_bytes_of_a_real_approvers_list() {
        use anchor_lang::AccountSerialize;
        use family_vault::state::TrustedApprovers;

        let mut entries = [anchor_lang::prelude::Pubkey::default(); MAX_APPROVERS];
        for (slot, entry) in entries.iter_mut().enumerate() {
            *entry = anchor_lang::prelude::Pubkey::new_from_array([slot as u8 + 1; 32]);
        }
        let list = TrustedApprovers {
            vault: anchor_lang::prelude::Pubkey::new_from_array([0xBB; 32]),
            count: MAX_APPROVERS as u8,
            entries,
        };
        let mut bytes = Vec::new();
        list.try_serialize(&mut bytes).unwrap();

        assert_eq!(bytes.len(), TrustedApprovers::SPACE);
        assert_eq!(approver_offset(0), 41);
        assert_eq!(approver_offset(4), 169);
        for slot in 0..MAX_APPROVERS {
            let at = approver_offset(slot);
            assert_eq!(&bytes[at..at + 32], &[slot as u8 + 1; 32], "slot {slot}");
        }
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
