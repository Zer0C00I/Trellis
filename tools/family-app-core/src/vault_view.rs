//! Read-only view of one vault for the UI: balance, limits, payees,
//! guardians (trusted approvers) and pending over-limit requests, all
//! decoded from chain with the real `family_vault` account types.
//!
//! The decoding and assembly are pure ([`assemble_view`]) and tested
//! offline; only [`fetch_vault_view`] and [`next_free_nonce`] touch the
//! network. Everything read from chain is untrusted: payee names are
//! sanitized, counts are clamped, and the account owner and discriminator
//! are checked before any field is believed.
//!
//! RPC calls made by [`fetch_vault_view`] (5 in total):
//! 1. `get_multiple_accounts([vault, beneficiary list, approvers])`
//! 2. `get_account_data(vault token account)`
//! 3. `get_account_data(mint)`
//! 4. `get_multiple_accounts(request PDAs for nonces 0..100)`
//! 5. `get_multiple_accounts(request PDAs for nonces 100..200)`

use crate::error::{map_client_error, ActionError};
use anchor_lang::AccountDeserialize;
use family_vault::state::{
    BeneficiaryList, PendingWithdrawal, TrustedApprovers, Vault, BENEFICIARY_NAME_LEN,
    MAX_APPROVERS, MAX_BENEFICIARIES,
};
use family_vault_client::{
    approvers_pda, beneficiary_list_pda, decode_mint_decimals, request_pda, to_client,
};
use solana_client::rpc_client::RpcClient;
use solana_pubkey::Pubkey;
use spl_token::solana_program::program_pack::Pack;

/// How many request nonces `0..NONCE_SCAN_LIMIT` are inspected.
///
/// This window is a deliberate MVP limit. `PendingWithdrawal` does not store
/// its nonce (it lives only in the PDA seeds), nonces are chosen by the
/// caller and are never reused (executed requests keep their account), so
/// finding "pending requests" and "a free nonce" means probing PDAs. Probing
/// a bounded, small window keeps the cost at two `get_multiple_accounts`
/// calls. A vault that has used all 200 nonces reports an error instead of
/// silently hiding older requests; lifting the limit needs a program-side
/// index or a `getProgramAccounts` scan.
pub const NONCE_SCAN_LIMIT: u64 = 200;

/// `getMultipleAccounts` accepts at most 100 keys per call.
const NONCE_CHUNK: u64 = 100;

/// An approved recipient, as shown to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payee {
    /// Sanitized display name (see [`sanitize_name`]); never raw chain text.
    pub name: String,
    pub address: Pubkey,
    /// Unix time from which this payee may be paid (48h after being added,
    /// unless added urgently).
    pub active_after: i64,
}

impl Payee {
    /// Whether the payee may be paid at `now_unix`.
    pub fn is_active(&self, now_unix: i64) -> bool {
        now_unix >= self.active_after
    }
}

/// An over-limit withdrawal request that still awaits approval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRequest {
    /// Pass this to `actions::approve_withdrawal`.
    pub nonce: u64,
    /// The request's PDA.
    pub address: Pubkey,
    pub to: Pubkey,
    /// Base units.
    pub amount: u64,
    pub requested_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultView {
    pub vault: Pubkey,
    pub owner: Pubkey,
    pub is_pool: bool,
    /// Base units per day; `None` when the vault is unlimited
    /// (`daily_limit == u64::MAX`). The program only enforces the limit on
    /// pool vaults, so a UI should look at `is_pool` too.
    pub daily_limit: Option<u64>,
    /// Base units spent today. The program stores a counter that is only
    /// reset by the next withdrawal on a new UTC day, so this is the
    /// effective value: 0 when the stored day is not today.
    pub spent_today: u64,
    pub mint: Pubkey,
    pub decimals: u32,
    /// Base units held by the vault's token account.
    pub balance: u64,
    pub payees: Vec<Payee>,
    /// Trusted approvers; empty when the approvers account does not exist yet.
    pub guardians: Vec<Pubkey>,
    /// Unexecuted requests among nonces `0..NONCE_SCAN_LIMIT`, by nonce.
    pub pending: Vec<PendingRequest>,
    /// The smallest nonce in the window with no request account. Pass it to
    /// `actions::request_withdrawal`, store it, and reuse it on retry.
    pub next_free_nonce: u64,
}

/// Turns untrusted on-chain name bytes into text safe to show or log.
///
/// Only the first `name_len` bytes are used (`name_len` is clamped to the
/// buffer, so a hostile length cannot index out of range), decoded as UTF-8
/// lossily. Control characters and invisible direction/format characters
/// (which could fake line breaks or reorder text) become U+FFFD.
pub fn sanitize_name(name: &[u8; BENEFICIARY_NAME_LEN], name_len: u8) -> String {
    let len = (name_len as usize).min(BENEFICIARY_NAME_LEN);
    String::from_utf8_lossy(&name[..len])
        .chars()
        .map(|c| if is_unsafe_char(c) { '\u{FFFD}' } else { c })
        .collect()
}

fn is_unsafe_char(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{200B}'..='\u{200F}'   // zero-width and direction marks
                | '\u{2028}'..='\u{202E}' // line/paragraph separators, bidi embeddings/overrides
                | '\u{2060}'..='\u{2069}' // word joiner, invisible operators, bidi isolates
                | '\u{FEFF}'
        )
}

/// Checks the owner and the Anchor discriminator, then decodes.
fn decode_account<T: AccountDeserialize>(
    owner: &Pubkey,
    data: &[u8],
    program_id: &Pubkey,
    what: &str,
) -> Result<T, ActionError> {
    if owner != program_id {
        return Err(ActionError::Rejected(format!(
            "{what} is not owned by the family-vault program"
        )));
    }
    T::try_deserialize(&mut &data[..])
        .map_err(|e| ActionError::Rejected(format!("decoding {what}: {e}")))
}

/// The amount held by an SPL token account, and its mint.
fn decode_token_account(data: &[u8]) -> Result<(Pubkey, u64), ActionError> {
    let account = spl_token::state::Account::unpack(data)
        .map_err(|e| ActionError::Rejected(format!("decoding the vault token account: {e}")))?;
    Ok((account.mint, account.amount))
}

/// Index of the first empty slot, which is the nonce itself.
fn first_free<T>(slots: &[Option<T>]) -> Option<u64> {
    slots.iter().position(Option::is_none).map(|i| i as u64)
}

fn too_many_requests() -> ActionError {
    ActionError::Rejected("too many withdrawal requests on this vault".to_string())
}

/// Everything [`assemble_view`] needs, already fetched and decoded.
pub struct ViewInputs {
    pub program_id: Pubkey,
    pub vault_pubkey: Pubkey,
    pub vault: Vault,
    pub list: BeneficiaryList,
    /// `None` when the approvers account does not exist yet.
    pub approvers: Option<TrustedApprovers>,
    /// Index = nonce; `None` = no account at that request PDA.
    pub request_slots: Vec<Option<PendingWithdrawal>>,
    pub token_balance: u64,
    pub mint: Pubkey,
    pub decimals: u32,
    /// Current unix time, for the effective `spent_today`.
    pub now_unix: i64,
}

/// Pure assembly of a [`VaultView`] from decoded chain data. Counts read
/// from chain are clamped to the fixed array sizes, never trusted.
pub fn assemble_view(input: ViewInputs) -> Result<VaultView, ActionError> {
    let ViewInputs {
        program_id,
        vault_pubkey,
        vault,
        list,
        approvers,
        request_slots,
        token_balance,
        mint,
        decimals,
        now_unix,
    } = input;

    let payees = list
        .entries
        .iter()
        .take((list.count as usize).min(MAX_BENEFICIARIES))
        .map(|e| Payee {
            name: sanitize_name(&e.name, e.name_len),
            address: to_client(e.address),
            active_after: e.active_after,
        })
        .collect();

    let guardians = approvers
        .map(|a| {
            a.entries
                .iter()
                .take((a.count as usize).min(MAX_APPROVERS))
                .copied()
                .map(to_client)
                .collect()
        })
        .unwrap_or_default();

    let next_free_nonce = first_free(&request_slots).ok_or_else(too_many_requests)?;

    let pending = request_slots
        .iter()
        .enumerate()
        .filter_map(|(nonce, slot)| {
            let request = slot.as_ref().filter(|r| !r.executed)?;
            let nonce = nonce as u64;
            Some(PendingRequest {
                nonce,
                address: request_pda(&program_id, &vault_pubkey, nonce).0,
                to: to_client(request.to),
                amount: request.amount,
                requested_at: request.requested_at,
            })
        })
        .collect();

    let spent_today = if vault.last_reset_day == now_unix.div_euclid(86_400) {
        vault.spent_today
    } else {
        0
    };

    Ok(VaultView {
        vault: vault_pubkey,
        owner: to_client(vault.owner),
        is_pool: vault.is_pool,
        daily_limit: (vault.daily_limit != u64::MAX).then_some(vault.daily_limit),
        spent_today,
        mint,
        decimals,
        balance: token_balance,
        payees,
        guardians,
        pending,
        next_free_nonce,
    })
}

/// Fetches request slots for nonces `0..limit` in chunks of 100, stopping
/// early once `stop_at_first_free` and a free slot was seen.
fn fetch_request_accounts(
    rpc: &RpcClient,
    program_id: &Pubkey,
    vault: &Pubkey,
    stop_at_first_free: bool,
) -> Result<Vec<Option<PendingWithdrawal>>, ActionError> {
    let mut slots = Vec::with_capacity(NONCE_SCAN_LIMIT as usize);
    for start in (0..NONCE_SCAN_LIMIT).step_by(NONCE_CHUNK as usize) {
        let keys: Vec<Pubkey> = (start..start + NONCE_CHUNK)
            .map(|nonce| request_pda(program_id, vault, nonce).0)
            .collect();
        let accounts = rpc.get_multiple_accounts(&keys).map_err(map_client_error)?;
        if accounts.len() != keys.len() {
            return Err(ActionError::Rejected(
                "the node returned an unexpected number of accounts".to_string(),
            ));
        }
        for account in accounts {
            slots.push(match account {
                None => None,
                Some(a) => Some(decode_account::<PendingWithdrawal>(
                    &a.owner,
                    &a.data,
                    program_id,
                    "a withdrawal request",
                )?),
            });
        }
        if stop_at_first_free && slots.iter().any(Option::is_none) {
            break;
        }
    }
    Ok(slots)
}

/// The smallest request nonce with no account yet, for a new
/// `request_withdrawal`. Looks at nonces `0..NONCE_SCAN_LIMIT`
/// (see that constant for why) and returns
/// `Rejected("too many withdrawal requests on this vault")` when all are taken.
/// One `get_multiple_accounts` call, two when the first 100 are all taken.
pub fn next_free_nonce(
    rpc: &RpcClient,
    program_id: &Pubkey,
    vault: &Pubkey,
) -> Result<u64, ActionError> {
    let slots = fetch_request_accounts(rpc, program_id, vault, true)?;
    first_free(&slots).ok_or_else(too_many_requests)
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Reads everything a UI needs to show one vault. A missing vault or
/// payee list is an error; a missing approvers account just means no
/// guardians yet. See the module docs for the exact RPC calls.
pub fn fetch_vault_view(
    rpc: &RpcClient,
    program_id: &Pubkey,
    vault: &Pubkey,
) -> Result<VaultView, ActionError> {
    let keys = [
        *vault,
        beneficiary_list_pda(program_id, vault).0,
        approvers_pda(program_id, vault).0,
    ];
    let mut accounts = rpc.get_multiple_accounts(&keys).map_err(map_client_error)?;
    if accounts.len() != keys.len() {
        return Err(ActionError::Rejected(
            "the node returned an unexpected number of accounts".to_string(),
        ));
    }
    let approvers_account = accounts.pop().flatten();
    let list_account = accounts
        .pop()
        .flatten()
        .ok_or_else(|| ActionError::Rejected("the vault has no payee list".to_string()))?;
    let vault_account = accounts
        .pop()
        .flatten()
        .ok_or_else(|| ActionError::Rejected("vault not found".to_string()))?;

    let vault_data: Vault = decode_account(
        &vault_account.owner,
        &vault_account.data,
        program_id,
        "the vault",
    )?;
    let list: BeneficiaryList = decode_account(
        &list_account.owner,
        &list_account.data,
        program_id,
        "the payee list",
    )?;
    let approvers = approvers_account
        .map(|a| {
            decode_account::<TrustedApprovers>(&a.owner, &a.data, program_id, "the approvers list")
        })
        .transpose()?;

    let token_account = to_client(vault_data.vault_token_account);
    let token_data = rpc
        .get_account_data(&token_account)
        .map_err(map_client_error)?;
    let (mint, token_balance) = decode_token_account(&token_data)?;
    let mint_data = rpc.get_account_data(&mint).map_err(map_client_error)?;
    let decimals = decode_mint_decimals(&mint_data).map_err(ActionError::Rejected)?;

    let request_slots = fetch_request_accounts(rpc, program_id, vault, false)?;

    assemble_view(ViewInputs {
        program_id: *program_id,
        vault_pubkey: *vault,
        vault: vault_data,
        list,
        approvers,
        request_slots,
        token_balance,
        mint,
        decimals,
        now_unix: now_unix(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::AccountSerialize;
    use family_vault::state::Beneficiary;
    use family_vault_client::to_anchor;
    use spl_token::solana_program::program_option::COption;

    const DAY: i64 = 86_400;

    fn key(byte: u8) -> Pubkey {
        Pubkey::new_from_array([byte; 32])
    }

    fn name_bytes(text: &[u8]) -> [u8; BENEFICIARY_NAME_LEN] {
        let mut name = [0u8; BENEFICIARY_NAME_LEN];
        name[..text.len()].copy_from_slice(text);
        name
    }

    fn vault(daily_limit: u64) -> Vault {
        Vault {
            owner: to_anchor(key(1)),
            vault_token_account: to_anchor(key(2)),
            daily_limit,
            spent_today: 40,
            last_reset_day: 10,
            is_pool: true,
            bump: 255,
            seed_owner: to_anchor(key(1)),
        }
    }

    fn list(entries: &[(&[u8], u8, i64)], count: u8) -> BeneficiaryList {
        let mut l = BeneficiaryList {
            vault: to_anchor(key(9)),
            count,
            entries: [Beneficiary::default(); MAX_BENEFICIARIES],
        };
        for (i, (name, len, active_after)) in entries.iter().enumerate() {
            l.entries[i] = Beneficiary {
                address: to_anchor(key(100 + i as u8)),
                name: name_bytes(name),
                name_len: *len,
                added_at: 0,
                active_after: *active_after,
            };
        }
        l
    }

    fn approvers(count: u8) -> TrustedApprovers {
        let mut a = TrustedApprovers {
            vault: to_anchor(key(9)),
            count,
            entries: [Default::default(); MAX_APPROVERS],
        };
        for i in 0..MAX_APPROVERS {
            a.entries[i] = to_anchor(key(50 + i as u8));
        }
        a
    }

    fn request(amount: u64, executed: bool) -> PendingWithdrawal {
        PendingWithdrawal {
            vault: to_anchor(key(9)),
            amount,
            to: to_anchor(key(77)),
            requested_at: 1234,
            approved_by: None,
            executed,
            bump: 254,
        }
    }

    fn inputs(slots: Vec<Option<PendingWithdrawal>>) -> ViewInputs {
        ViewInputs {
            program_id: key(3),
            vault_pubkey: key(9),
            vault: vault(1_000),
            list: list(&[(b"Anna", 4, 0)], 1),
            approvers: Some(approvers(2)),
            request_slots: slots,
            token_balance: 5_000_000,
            mint: key(4),
            decimals: 6,
            now_unix: 10 * DAY + 5,
        }
    }

    fn slots_with(used: &[usize]) -> Vec<Option<PendingWithdrawal>> {
        (0..NONCE_SCAN_LIMIT as usize)
            .map(|i| used.contains(&i).then(|| request(1, false)))
            .collect()
    }

    #[test]
    fn payees_are_only_the_first_count_entries() {
        let mut i = inputs(slots_with(&[]));
        i.list = list(&[(b"Anna", 4, 0), (b"Boris", 5, 99), (b"Ghost", 5, 0)], 2);
        let view = assemble_view(i).unwrap();
        assert_eq!(view.payees.len(), 2);
        assert_eq!(view.payees[0].name, "Anna");
        assert_eq!(view.payees[0].address, key(100));
        assert_eq!(view.payees[1].name, "Boris");
        assert_eq!(view.payees[1].active_after, 99);
    }

    #[test]
    fn a_garbage_payee_count_is_clamped_not_a_panic() {
        let mut i = inputs(slots_with(&[]));
        i.list = list(&[(b"Anna", 4, 0)], 255);
        let view = assemble_view(i).unwrap();
        assert_eq!(view.payees.len(), MAX_BENEFICIARIES);
    }

    #[test]
    fn payee_activity_follows_active_after() {
        let p = Payee {
            name: "x".into(),
            address: key(1),
            active_after: 100,
        };
        assert!(!p.is_active(99));
        assert!(p.is_active(100));
        assert!(p.is_active(101));
    }

    #[test]
    fn unlimited_and_limited_daily_limits() {
        let mut i = inputs(slots_with(&[]));
        i.vault = vault(u64::MAX);
        assert_eq!(assemble_view(i).unwrap().daily_limit, None);
        let mut i = inputs(slots_with(&[]));
        i.vault = vault(2_500_000);
        let view = assemble_view(i).unwrap();
        assert_eq!(view.daily_limit, Some(2_500_000));
        assert_eq!(view.owner, key(1));
        assert!(view.is_pool);
        assert_eq!(view.balance, 5_000_000);
        assert_eq!(view.decimals, 6);
        assert_eq!(view.mint, key(4));
        assert_eq!(view.vault, key(9));
    }

    #[test]
    fn spent_today_is_zero_once_the_stored_day_has_passed() {
        let view = assemble_view(inputs(slots_with(&[]))).unwrap();
        assert_eq!(view.spent_today, 40); // last_reset_day 10 == today
        let mut i = inputs(slots_with(&[]));
        i.now_unix = 11 * DAY;
        assert_eq!(assemble_view(i).unwrap().spent_today, 0);
    }

    #[test]
    fn guardians_follow_count_and_are_clamped() {
        let view = assemble_view(inputs(slots_with(&[]))).unwrap();
        assert_eq!(view.guardians, vec![key(50), key(51)]);
        let mut i = inputs(slots_with(&[]));
        i.approvers = Some(approvers(200));
        assert_eq!(assemble_view(i).unwrap().guardians.len(), MAX_APPROVERS);
    }

    #[test]
    fn a_missing_approvers_account_means_no_guardians() {
        let mut i = inputs(slots_with(&[]));
        i.approvers = None;
        assert!(assemble_view(i).unwrap().guardians.is_empty());
    }

    #[test]
    fn pending_excludes_executed_requests_and_carries_nonce_and_pda() {
        let mut slots = slots_with(&[]);
        slots[0] = Some(request(10, true));
        slots[1] = Some(request(20, false));
        slots[3] = Some(request(30, false));
        let view = assemble_view(inputs(slots)).unwrap();
        let nonces: Vec<u64> = view.pending.iter().map(|p| p.nonce).collect();
        assert_eq!(nonces, vec![1, 3]);
        assert_eq!(view.pending[0].amount, 20);
        assert_eq!(view.pending[1].amount, 30);
        assert_eq!(view.pending[0].to, key(77));
        assert_eq!(view.pending[0].requested_at, 1234);
        assert_eq!(view.pending[0].address, request_pda(&key(3), &key(9), 1).0);
        assert_eq!(view.pending[1].address, request_pda(&key(3), &key(9), 3).0);
    }

    #[test]
    fn next_free_nonce_is_the_smallest_gap() {
        let view = assemble_view(inputs(slots_with(&[0, 1, 3]))).unwrap();
        assert_eq!(view.next_free_nonce, 2);
        let view = assemble_view(inputs(slots_with(&[]))).unwrap();
        assert_eq!(view.next_free_nonce, 0);
        let all_but_last: Vec<usize> = (0..NONCE_SCAN_LIMIT as usize - 1).collect();
        let view = assemble_view(inputs(slots_with(&all_but_last))).unwrap();
        assert_eq!(view.next_free_nonce, NONCE_SCAN_LIMIT - 1);
    }

    #[test]
    fn executed_requests_still_occupy_their_nonce() {
        let mut slots = slots_with(&[]);
        slots[0] = Some(request(10, true));
        assert_eq!(assemble_view(inputs(slots)).unwrap().next_free_nonce, 1);
    }

    #[test]
    fn all_nonces_taken_is_an_error() {
        let all: Vec<usize> = (0..NONCE_SCAN_LIMIT as usize).collect();
        match assemble_view(inputs(slots_with(&all))) {
            Err(ActionError::Rejected(m)) => {
                assert_eq!(m, "too many withdrawal requests on this vault")
            }
            other => panic!("expected the too-many error, got {other:?}"),
        }
        assert_eq!(first_free::<u8>(&[]), None);
        assert_eq!(first_free(&[Some(1), None, Some(2)]), Some(1));
    }

    #[test]
    fn names_use_only_name_len_bytes_and_survive_garbage_len() {
        assert_eq!(sanitize_name(&name_bytes(b"Anna"), 4), "Anna");
        assert_eq!(sanitize_name(&name_bytes(b"Anna"), 2), "An");
        assert_eq!(sanitize_name(&name_bytes(b"Anna"), 0), "");
        // 255 is clamped to the 32-byte buffer: the 28 zero bytes become U+FFFD.
        let garbage = sanitize_name(&name_bytes(b"Anna"), 255);
        assert!(garbage.starts_with("Anna"));
        assert_eq!(garbage.chars().count(), 32);
        assert!(garbage[4..].chars().all(|c| c == '\u{FFFD}'));
    }

    #[test]
    fn invalid_utf8_is_replaced_not_dropped_or_panicking() {
        let name = name_bytes(&[b'A', 0xFF, 0xFE, b'B']);
        assert_eq!(sanitize_name(&name, 4), "A\u{FFFD}\u{FFFD}B");
        // A multi-byte character cut in half by name_len.
        let name = name_bytes("é".as_bytes());
        assert_eq!(sanitize_name(&name, 1), "\u{FFFD}");
        assert_eq!(sanitize_name(&name, 2), "é");
    }

    #[test]
    fn control_and_bidi_characters_cannot_forge_text() {
        let name = name_bytes(b"A\nB\r\x1b[31mC\t\0");
        let out = sanitize_name(&name, 12);
        assert!(!out.chars().any(|c| c.is_control()), "{out:?}");
        assert!(out.starts_with('A') && out.contains('B') && out.contains('C'));
        let sneaky = "a\u{202E}b\u{2028}c\u{200B}d\u{FEFF}";
        let out = sanitize_name(&name_bytes(sneaky.as_bytes()), sneaky.len() as u8);
        assert_eq!(out, "a\u{FFFD}b\u{FFFD}c\u{FFFD}d\u{FFFD}");
        // Ordinary non-ASCII names are untouched.
        let cyr = "Мама";
        assert_eq!(
            sanitize_name(&name_bytes(cyr.as_bytes()), cyr.len() as u8),
            cyr
        );
    }

    fn serialized<T: AccountSerialize>(account: &T) -> Vec<u8> {
        let mut data = Vec::new();
        account.try_serialize(&mut data).unwrap();
        data
    }

    #[test]
    fn real_account_bytes_round_trip_through_decode_account() {
        let program = key(3);
        let data = serialized(&vault(77));
        let v: Vault = decode_account(&program, &data, &program, "the vault").unwrap();
        assert_eq!(v.daily_limit, 77);
        let data = serialized(&request(5, false));
        let r: PendingWithdrawal = decode_account(&program, &data, &program, "r").unwrap();
        assert_eq!(r.amount, 5);
    }

    #[test]
    fn decode_account_rejects_foreign_owner_and_wrong_discriminator() {
        let program = key(3);
        let data = serialized(&vault(77));
        assert!(decode_account::<Vault>(&key(8), &data, &program, "the vault").is_err());
        // A BeneficiaryList's bytes are not a Vault.
        let other = serialized(&list(&[], 0));
        assert!(decode_account::<Vault>(&program, &other, &program, "the vault").is_err());
        assert!(decode_account::<Vault>(&program, &[], &program, "the vault").is_err());
    }

    #[test]
    fn token_account_amount_and_mint_are_decoded() {
        let account = spl_token::state::Account {
            mint: key(4),
            owner: key(1),
            amount: 123_456,
            delegate: COption::None,
            state: spl_token::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };
        let mut data = vec![0u8; spl_token::state::Account::LEN];
        spl_token::state::Account::pack(account, &mut data).unwrap();
        assert_eq!(decode_token_account(&data).unwrap(), (key(4), 123_456));
        assert!(decode_token_account(&data[..10]).is_err());
    }

    // Requires a live RPC endpoint and a real vault. Run manually with
    // `VAULT_VIEW_TEST_VAULT=<pubkey> cargo test -p family-app-core vault_view::tests::fetches_a_real_vault_view -- --ignored --nocapture`
    #[test]
    #[ignore = "needs a live RPC endpoint and a real vault"]
    fn fetches_a_real_vault_view() {
        let rpc =
            solana_client::rpc_client::RpcClient::new("https://api.devnet.solana.com".to_string());
        let program_id: Pubkey = "9ozRBpTmDJJzcmJhKEbfvsoe7qqo2584bus1rSMG4zwJ"
            .parse()
            .unwrap();
        let vault: Pubkey = std::env::var("VAULT_VIEW_TEST_VAULT")
            .expect("set VAULT_VIEW_TEST_VAULT to a vault pubkey")
            .parse()
            .unwrap();
        let view = fetch_vault_view(&rpc, &program_id, &vault).unwrap();
        println!("{view:#?}");
        assert_eq!(view.vault, vault);
    }
}
