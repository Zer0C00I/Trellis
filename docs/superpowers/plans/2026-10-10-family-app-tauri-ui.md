# Family App (Tauri shell + UI) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A runnable Trellis Family desktop app: the family member opens it, hands their key to a kiosk during its first-run setup (or opens a family account of their own), then sees balance and history, deposits, sends to approved payees, adds payees and guardians, and approves over-limit transfers, all in plain Russian with no blockchain words.

**Architecture:** `tools/family-app-core` (already built) does all chain work. A new Tauri 2 crate `family-app/src-tauri` is a thin IPC layer: pure, unit-tested modules (`text`, `view`, `pending`) decide every sentence and every permission; thin `commands` call one core function on a blocking thread. `family-app/ui` is dependency-free HTML/CSS/JS (ES modules) that only displays what Rust returns and never does money math. A scripted mock backend lets every screen run in a plain browser and under a headless smoke test.

**Tech Stack:** Rust, Tauri `=2.12.2` (+ `tauri-build =2.7.1`), `family-app-core`, vanilla JavaScript ES modules, `node --test`, headless Chromium over the DevTools protocol (smoke test).

**Spec:** `docs/superpowers/specs/2026-09-23-family-app-design.md` (executors read both). Where this plan deliberately differs from the spec, see "Deviations from the spec" below; each one was forced by something verified while preparing the plan.

## Global Constraints

- **Platforms:** Linux desktop is built and verified. Android and Windows are spec targets but are NOT built or tested here (mobile needs the Android SDK/NDK and a camera-scanner plugin; see "Explicitly not in this plan"). iOS is out of scope.
- **Language:** every sentence the person reads is Russian, plain bank language. Never show the words "vault", "PDA", "mint", "approver", "beneficiary" or "nonce" in the UI (spec, "MVP Scope"). Technical text appears only inside a folded "Подробности" element.
- **Closed address book:** Send never has a free-text address field; the recipient is picked from the payee list (spec, "Screens" #4).
- **No seed phrase, no wallet import** (spec, "Identity & Bootstrap"). The key is generated on first launch by `family-app-core::identity` and never shown or exported.
- **Network:** Devnet only. Program ID is `family_vault::ID` (never hard-coded in JS); the token is Circle Devnet USDC `4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU`. `TRELLIS_RPC_URL` may override the endpoint for development.
- **Refresh:** polling, no push (spec, "Data Flow"). Every 60 s while visible plus on returning to the foreground, and a refresh never rebuilds the screen while the person is typing. (The spec says 30 s; the public Devnet node rate-limits, see deviations.)
- **Money safety:** JavaScript never parses, adds or compares amounts. Amounts are typed as text and parsed exactly in Rust with `family_vault_client::parse_decimal_amount`; every amount shown is formatted by Rust; `u64` values never cross to JS as numbers.
- **Unconfirmed operations:** while a sent operation's outcome is unknown, every command that moves money refuses to run (`commands::ensure_no_pending`). The record survives restarts.
- **Untrusted text:** names and addresses come from the chain. The UI builds DOM with `textContent`/text nodes only (never `innerHTML`); Rust cleans names with `family_app_core::vault_view::sanitize_text`.
- **Rust:** `[lints.rust] unsafe_code = "deny"`, `dead_code = "deny"` (a crate-level `#![allow(dead_code)]` is allowed ONLY in Tasks 2-3 and must be removed in Task 4). Commands never panic and never `unwrap` chain or user data. Exact `=` pins for the Tauri crates, matching the workspace convention. `cargo fmt` and `cargo clippy --all-targets -- -D warnings` must be clean for the package.
- **Commits:** work directly on `master` (project convention). Do not add attribution lines to commit messages. Never `git add` anything under `target/`, `node_modules/` or `family-app/src-tauri/gen/`.

---

## Deviations from the spec (decided, with the evidence)

1. **Discovery takes 2 requests, not 15.** Measured against the public Devnet node: it allows 10 `getProgramAccounts` calls per 10 seconds per IP (`x-ratelimit-method-limit: 10`, then HTTP 429 with `retry-after: 10`). The spec's 15 filtered calls therefore always hit the limit and a launch took 20-22 seconds (measured), and parallelising did not help (429s). One request per list kind filtered only by exact account size, with the "is it me" test done on the client, takes 135 ms. Scale note: each list is 851 or 201 bytes, so even a thousand vaults is about a megabyte; a large deployment needs an indexer or a provider-side filter. Task 1 replaces the committed 15-call implementation and corrects the spec text again.
2. **The app also finds vaults it OWNS.** Being the owner puts a key in no payee or guardian list, so discovery by list membership can never find a family account the person opened with "Открыть семейный счёт". Task 1 adds `discover_owned_vaults` (the key's pool and personal PDAs).
3. **A minimal vault picker is included.** The spec says one vault at a time with no switcher. But a person who is a guardian on a relative's vault and also opens their own family account would otherwise be unable to reach one of the two. The picker is one `<select>` shown only when there are two or more vaults.
4. **"Добавить получателя срочно" (urgent add) is NOT in this plan.** It needs two DIFFERENT guardians to sign the same transaction; this app holds one key on one device. It needs a co-signing transport (a partially signed transaction passed between two phones), which is its own design. Normal "add payee" is offered, but only to the vault's owner (the program checks `signer == vault.owner`).
5. **Most actions are role-gated.** Verified against the program: send, add payee, add guardian and request-over-limit require the OWNER's signature; approving requires a trusted approver; depositing is open to anyone; a personal vault that already has a guardian needs that guardian to co-sign a second one (not possible on one device, so not offered). `view::permissions` encodes this once and the commands re-check it.
6. **Fallback 6-digit code is supported.** The spec requires it ("Ввести код вручную") but `family-app-core` only accepted the QR text. Task 1 adds `pair_with_kiosk_pin`.
7. **Camera QR scanning is NOT in this plan.** `tauri-plugin-barcode-scanner` (2.5.1) is mobile-only, so it belongs with the Android build. On desktop the person pastes the QR text (`fvpair1:...`) or types the 6 digits; the input accepts either.
8. **A new key has no SOL for fees.** Every transaction is paid in SOL by the app's own key, and a fresh key has none (the spec is silent on this). The "Я" tab shows the address, the SOL balance and, on Devnet, a "get test SOL" button (Task 1 adds `family_app_core::wallet`). The public faucet is rate-limited, so the screen also tells the person they can fund the address by hand.
9. **History shows 10 entries, refresh is 60 s.** Each history entry costs one `getTransaction`; keeping it small and the refresh slow keeps the app inside the public node's limits.

## Explicitly not in this plan

- Android and Windows builds, camera QR scanning (`tauri-plugin-barcode-scanner`), app icons/branding beyond a placeholder icon.
- Urgent add / any multi-signer flow (needs a co-signing transport).
- Owner rotation screens, notifications or a background watcher, iOS.
- Encrypting the identity key file at rest (a known prototype limitation; the website states it honestly).
- Verifying Bluetooth pairing against a physical kiosk and the money actions against a funded Devnet key: these are the MANUAL checks in Task 7. Nothing in this plan exercises a real kiosk or sends real Devnet transactions.

## File Structure

```
tools/family-app-core/src/
  discovery.rs      REPLACE  2 size-filtered requests + client-side match; discover_owned_vaults (Task 1)
  wallet.rs         CREATE   SOL balance, Devnet faucet (Task 1)
  pairing.rs        MODIFY   pair_with_kiosk_pin, BadPin (Task 1)
  vault_view.rs     MODIFY   sanitize_text (Task 1)
  lib.rs            MODIFY   pub mod wallet (Task 1)
family-app/
  src-tauri/
    Cargo.toml, build.rs, tauri.conf.json, icons/icon.png            (Task 2)
    src/main.rs, lib.rs, config.rs, state.rs                          (Task 2; lib.rs grows in Tasks 3-4)
    src/dto.rs, text.rs, view.rs                                      (Task 3, pure and tested)
    src/pending.rs, commands/{mod,read,actions,pairing}.rs            (Task 4)
  ui/
    package.json, dom.js, errors.js, input.js, format.js, router.js, test/pure.test.js   (Task 5, pure JS + node tests)
    index.html, styles.css, api.js, mock-api.js, state.js, components.js, app.js,
    screens/{first,home,send,people,approvals,me}.js                  (Task 6)
    tools/smoke.mjs                                                   (Task 7)
  README.md                                                           (Task 7)
Cargo.toml  (workspace member)   .gitignore  (+ generated Tauri files)   docs/superpowers/specs/2026-09-23-family-app-design.md  (Vault Discovery)
```

Linux build prerequisites (already present on the development machine): `webkit2gtk-4.1`, `javascriptcoregtk-4.1`, `libsoup-3.0`, `gtk+-3.0`, `librsvg-2.0`, `openssl` development files (all found by `pkg-config`).

---

### Task 1: Core additions (`family-app-core`): fast discovery, owned vaults, fallback code, wallet

**Files:**
- Replace: `tools/family-app-core/src/discovery.rs`
- Create: `tools/family-app-core/src/wallet.rs`
- Modify: `tools/family-app-core/src/pairing.rs`, `tools/family-app-core/src/vault_view.rs`, `tools/family-app-core/src/lib.rs`
- Modify: `docs/superpowers/specs/2026-09-23-family-app-design.md` ("Vault Discovery")

**Interfaces:**
- Consumes (existing): `family_vault_client::{vault_pda, to_anchor, to_client}`, `crate::error::{ActionError, map_client_error}`, `family_vault_client::pairing::{auth_from_pin, AuthField, ...}`.
- Produces (used by Task 4):
  - `discovery::discover_my_vaults(rpc: &RpcClient, program_id: &Pubkey, me: &Pubkey) -> Result<Vec<Pubkey>, ActionError>` (same signature, now 2 requests)
  - `discovery::owned_vault_addresses(program_id: &Pubkey, owner: &Pubkey) -> [Pubkey; 2]` and `discovery::discover_owned_vaults(rpc: &RpcClient, program_id: &Pubkey, owner: &Pubkey) -> Result<Vec<Pubkey>, ActionError>`
  - `wallet::LOW_SOL_LAMPORTS: u64`, `wallet::sol_balance(rpc: &RpcClient, address: &Pubkey) -> Result<u64, ActionError>`, `wallet::request_devnet_airdrop(rpc: &RpcClient, address: &Pubkey, lamports: u64) -> Result<(), ActionError>`
  - `pairing::pair_with_kiosk_pin(pin: &str, name: &str, my_pubkey: Pubkey) -> Result<(), PairingError>` and the new variant `PairingError::BadPin`
  - `vault_view::sanitize_text(text: &str) -> String`

Why this task exists: see deviations 1, 2, 6 and 8. Everything below was compiled and tested in a copy of the real workspace before this plan was written (`cargo test -p family-app-core`: 86 passed, 5 ignored; clippy and fmt clean).

- [ ] **Step 1: Replace `tools/family-app-core/src/discovery.rs` with exactly this**

```rust
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
```

- [ ] **Step 2: Run the discovery tests**

Run: `cargo test -p family-app-core discovery:: 2>&1 | tail -8`
Expected: `test result: ok. 6 passed; 0 failed; 1 ignored`.

- [ ] **Step 3: Update the spec's "Vault Discovery" section**

In `docs/superpowers/specs/2026-09-23-family-app-design.md`, replace everything from the line `its pubkey participates in:` through the paragraph ending `problem — not needed for a hackathon MVP.` (the code block, the "multi-vault-capable" paragraph and the "`getProgramAccounts`'s filters are ANDed" paragraph) with:

````markdown
its pubkey participates in or owns:

```
getProgramAccounts(program_id, [dataSize(BeneficiaryList::SPACE)])   // 1 request
getProgramAccounts(program_id, [dataSize(TrustedApprovers::SPACE)])  // 1 request
keep every list whose live entries (the first `count`) contain my_pubkey,
collect its `vault` field, dedupe
getMultipleAccounts([pool_pda(me), personal_pda(me)])                // vaults I own
```

This is architecturally multi-vault-capable: one pubkey can appear in any
number of vaults' lists and can own up to two vaults. **MVP scope decision:
the UI shows one vault at a time**; when discovery finds more than one, a
small picker lets the person switch (needed because the same person can be
a guardian on a relative's vault and also own a family account).

`getProgramAccounts` filters are ANDed, so "is my key in any of the 10
payee slots" cannot be a single filtered call, and one filtered call per
slot (15 requests) was measured to be unusable on the public Devnet node:
it allows 10 `getProgramAccounts` calls per 10 seconds per IP, so the 11th
request is answered with HTTP 429 and `retry-after: 10`, making every
launch take 20 seconds or more (parallel requests only trigger more 429s).
Discovery therefore asks for every list of one kind, filtered only by
exact account size (2 requests, about 135 ms), and checks for the key on
the client. Each list is 851 (payees) or 201 (guardians) bytes, so this is
fine at MVP scale (a thousand vaults is about a megabyte). A deployment
with many vaults should put an indexer, or a provider that supports
server-side filtering, in front of the same two requests.
````

- [ ] **Step 4: Lint, then commit the discovery change**

Run: `cargo fmt -p family-app-core && cargo clippy -p family-app-core --all-targets -- -D warnings 2>&1 | tail -3`
Expected: no warnings or errors.

```bash
git add tools/family-app-core/src/discovery.rs docs/superpowers/specs/2026-09-23-family-app-design.md
git commit -m "fix(family-app-core): discover vaults with 2 requests, and find owned vaults

The public Devnet node allows 10 getProgramAccounts calls per 10 seconds,
so 15 filtered requests always hit HTTP 429 and took 20+ seconds. Ask for
every list of one kind by exact size and match the key on the client (135 ms).
Also find the vaults the key owns, which list membership can never reveal."
```

- [ ] **Step 5: Create `tools/family-app-core/src/wallet.rs`**

```rust
//! The app key's own SOL: balance for the "you need SOL for fees" hint, and
//! a Devnet-only faucet request. Every transaction the app sends is paid
//! for with SOL from this key, so a brand-new identity cannot do anything
//! until it has some.

use std::time::{Duration, Instant};

use solana_client::rpc_client::RpcClient;
use solana_pubkey::Pubkey;

use crate::error::{map_client_error, ActionError};

/// Below this the app tells the user to add SOL before doing anything that
/// costs a fee (about 0.01 SOL covers a few dozen transactions).
pub const LOW_SOL_LAMPORTS: u64 = 10_000_000;

const AIRDROP_WAIT: Duration = Duration::from_secs(30);
const AIRDROP_POLL: Duration = Duration::from_secs(1);

pub fn sol_balance(rpc: &RpcClient, address: &Pubkey) -> Result<u64, ActionError> {
    rpc.get_balance(address).map_err(map_client_error)
}

/// Requests `lamports` from the Devnet faucet and waits until the funds
/// arrive. Devnet only: on any other cluster the faucet call is refused by
/// the node. The public faucet is rate limited, so a refusal is common.
pub fn request_devnet_airdrop(
    rpc: &RpcClient,
    address: &Pubkey,
    lamports: u64,
) -> Result<(), ActionError> {
    let signature = rpc
        .request_airdrop(address, lamports)
        .map_err(map_client_error)?;
    let deadline = Instant::now() + AIRDROP_WAIT;
    loop {
        match rpc.confirm_transaction(&signature) {
            Ok(true) => return Ok(()),
            Ok(false) => {}
            Err(e) => return Err(map_client_error(e)),
        }
        if Instant::now() >= deadline {
            return Err(ActionError::Rejected(
                "the faucet request did not complete in time".to_string(),
            ));
        }
        std::thread::sleep(AIRDROP_POLL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_low_sol_threshold_is_a_sensible_fraction_of_one_sol() {
        const { assert!(LOW_SOL_LAMPORTS > 0 && LOW_SOL_LAMPORTS < 1_000_000_000) };
    }

    // Needs a Devnet endpoint and the public faucet (rate limited). Run with
    // `DEVNET_AIRDROP_ADDRESS=<pubkey> cargo test -p family-app-core wallet::tests::airdrop -- --ignored --nocapture`
    #[test]
    #[ignore = "needs Devnet and the public faucet"]
    fn airdrop_arrives() {
        let address: Pubkey = std::env::var("DEVNET_AIRDROP_ADDRESS")
            .expect("set DEVNET_AIRDROP_ADDRESS")
            .parse()
            .unwrap();
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let before = sol_balance(&rpc, &address).unwrap();
        request_devnet_airdrop(&rpc, &address, 100_000_000).unwrap();
        assert!(sol_balance(&rpc, &address).unwrap() > before);
    }
}
```

- [ ] **Step 6: Apply the pairing, vault_view and lib changes**

Save this as `/tmp/task1.patch` and run `git apply /tmp/task1.patch` from the repository root:

```diff
--- a/tools/family-app-core/src/pairing.rs
+++ b/tools/family-app-core/src/pairing.rs
@@ -12,8 +12,8 @@
 use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
 use btleplug::platform::{Manager, Peripheral};
 use family_vault_client::pairing::{
-    decode_qr_payload, encode_frame, FrameError, PairingRequest, PAIRING_CHARACTERISTIC_UUID_U128,
-    SERVICE_UUID_U128,
+    auth_from_pin, decode_qr_payload, encode_frame, AuthField, FrameError, PairingRequest,
+    PAIRING_CHARACTERISTIC_UUID_U128, SERVICE_UUID_U128,
 };
 use solana_pubkey::Pubkey;
 use uuid::Uuid;
@@ -33,6 +33,8 @@
     /// The scanned QR text is not a kiosk pairing code. No Bluetooth
     /// activity happened.
     BadQrText,
+    /// The fallback code is not exactly six digits.
+    BadPin,
     /// The name cannot be encoded (empty, too long, control characters).
     BadName(String),
     /// No Bluetooth adapter on this machine.
@@ -54,6 +56,7 @@
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
         match self {
             Self::BadQrText => write!(f, "not a valid kiosk pairing code"),
+            Self::BadPin => write!(f, "the code must be exactly six digits"),
             Self::BadName(e) => write!(f, "invalid name: {e}"),
             Self::NoAdapter => write!(f, "no Bluetooth adapter available"),
             Self::KioskNotFound => write!(f, "no kiosk in pairing mode found nearby"),
@@ -110,12 +113,30 @@
 /// reported before any Bluetooth activity.
 pub fn pair_with_kiosk(qr_text: &str, name: &str, my_pubkey: Pubkey) -> Result<(), PairingError> {
     let auth = decode_qr_payload(qr_text).ok_or(PairingError::BadQrText)?;
-    let frame = encode_frame(&PairingRequest {
+    pair_with_auth(auth, name, my_pubkey)
+}
+
+/// Same as [`pair_with_kiosk`] for the six-digit fallback code the kiosk
+/// shows under its QR (for a phone whose camera cannot scan). The code is
+/// guessable, so the kiosk limits wrong attempts and locks out for a while;
+/// a wrong code surfaces as `WriteRejected`.
+pub fn pair_with_kiosk_pin(pin: &str, name: &str, my_pubkey: Pubkey) -> Result<(), PairingError> {
+    let auth = auth_from_pin(pin.trim()).map_err(|_| PairingError::BadPin)?;
+    pair_with_auth(auth, name, my_pubkey)
+}
+
+/// The frame the kiosk expects, for any kind of credential.
+fn build_frame(auth: AuthField, name: &str, my_pubkey: Pubkey) -> Result<Vec<u8>, PairingError> {
+    encode_frame(&PairingRequest {
         auth,
         pubkey: my_pubkey,
         name: name.to_string(),
     })
-    .map_err(|e| PairingError::BadName(bad_name_reason(e).to_string()))?;
+    .map_err(|e| PairingError::BadName(bad_name_reason(e).to_string()))
+}
+
+fn pair_with_auth(auth: AuthField, name: &str, my_pubkey: Pubkey) -> Result<(), PairingError> {
+    let frame = build_frame(auth, name, my_pubkey)?;
 
     // A dedicated thread has no ambient tokio context, so `block_on` cannot
     // panic with "cannot start a runtime from within a runtime" even if the
@@ -232,6 +253,30 @@
     use solana_pubkey::Pubkey;
 
     #[test]
+    fn rejects_a_fallback_code_that_is_not_six_digits_before_any_bluetooth() {
+        for bad in ["", "12345", "1234567", "12345a", "١٢٣٤٥٦", "12 456"] {
+            let err = pair_with_kiosk_pin(bad, "Саша", Pubkey::new_from_array([1u8; 32]));
+            assert!(matches!(err, Err(PairingError::BadPin)), "{bad:?}");
+        }
+    }
+
+    #[test]
+    fn the_fallback_code_goes_into_the_auth_field_zero_padded() {
+        let auth = auth_from_pin("123456").unwrap();
+        let frame = build_frame(auth, "Саша", Pubkey::new_from_array([7u8; 32])).unwrap();
+        assert_eq!(&frame[..6], b"123456");
+        assert_eq!(&frame[6..16], &[0u8; 10]);
+        assert_eq!(&frame[16..48], &[7u8; 32]);
+    }
+
+    #[test]
+    fn surrounding_whitespace_around_the_fallback_code_is_ignored() {
+        // Only the BadPin check is offline-testable; a padded valid code must
+        // get past it (it then fails at Bluetooth, never with BadPin).
+        assert!(auth_from_pin(" 123456 ".trim()).is_ok());
+    }
+
+    #[test]
     fn rejects_qr_text_that_does_not_decode() {
         let err = pair_with_kiosk(
             "not a real pairing code",
--- a/tools/family-app-core/src/vault_view.rs
+++ b/tools/family-app-core/src/vault_view.rs
@@ -132,8 +132,14 @@
 /// sanitised: show the address too.
 pub fn sanitize_name(name: &[u8; BENEFICIARY_NAME_LEN], name_len: u8) -> String {
     let len = (name_len as usize).min(BENEFICIARY_NAME_LEN);
-    String::from_utf8_lossy(&name[..len])
-        .chars()
+    sanitize_text(&String::from_utf8_lossy(&name[..len]))
+}
+
+/// Replaces every character that could spoof or break UI text (see
+/// [`sanitize_name`]) with U+FFFD. For any text that came from the chain,
+/// such as a name decoded from transaction history.
+pub fn sanitize_text(text: &str) -> String {
+    text.chars()
         .map(|c| if is_unsafe_char(c) { '\u{FFFD}' } else { c })
         .collect()
 }
@@ -813,4 +819,11 @@
         println!("{view:#?}");
         assert_eq!(view.vault, vault);
     }
+
+    #[test]
+    fn sanitize_text_replaces_control_and_bidi_characters() {
+        assert_eq!(sanitize_text("Саша"), "Саша");
+        assert_eq!(sanitize_text("a\nb"), "a\u{FFFD}b");
+        assert_eq!(sanitize_text("x\u{202E}y"), "x\u{FFFD}y");
+    }
 }
--- a/tools/family-app-core/src/lib.rs
+++ b/tools/family-app-core/src/lib.rs
@@ -25,5 +25,6 @@
 pub mod identity;
 pub mod pairing;
 pub mod vault_view;
+pub mod wallet;
 
 pub use error::ActionError;
```

- [ ] **Step 7: Run everything for the package**

Run: `cargo fmt -p family-app-core && cargo test -p family-app-core 2>&1 | grep "test result" | head -1 && cargo clippy -p family-app-core --all-targets -- -D warnings 2>&1 | tail -3`
Expected: `test result: ok. 86 passed; 0 failed; 5 ignored`, then no clippy output.

- [ ] **Step 8: Commit**

```bash
git add tools/family-app-core
git commit -m "feat(family-app-core): fallback code pairing, wallet balance and faucet

pair_with_kiosk_pin (the spec's manual 6-digit code), wallet::{sol_balance,
request_devnet_airdrop} because every transaction is paid in SOL, and
vault_view::sanitize_text for text decoded from history."
```

---

### Task 2: Tauri crate scaffold (identity, config, shared state)

**Files:**
- Modify: `Cargo.toml` (workspace member), `.gitignore`
- Create: `family-app/src-tauri/{Cargo.toml,build.rs,tauri.conf.json,icons/icon.png}`, `family-app/src-tauri/src/{main.rs,lib.rs,config.rs,state.rs}`, `family-app/ui/index.html` (placeholder, replaced in Task 6)

**Interfaces:**
- Consumes: `family_app_core::identity::generate_or_load_identity(path: &Path) -> Result<Keypair, IdentityError>`.
- Produces (used by Tasks 3-4): `config::{rpc_url() -> String, program_id() -> Pubkey, usdc_mint() -> Pubkey, IDENTITY_FILE, PENDING_FILE, AIRDROP_LAMPORTS}`; `state::AppState { identity: Arc<Keypair>, rpc: Arc<RpcClient>, program_id: Pubkey, data_dir: PathBuf }` with `me()` and `pending_path()`; a running Tauri app that creates its identity file in the OS app-data directory on first launch.

Verified: this scaffold resolves in the real workspace without changing the version of any existing package in `Cargo.lock` (it adds the Tauri dependency tree, roughly 160 new packages), and compiles and passes clippy.

- [ ] **Step 1: Add the workspace member and ignore generated files**

In the root `Cargo.toml`, add `"family-app/src-tauri"` to `members` (after `"tools/family-app-core"`). Append to `.gitignore`:

```gitignore
# Tauri build output generated next to the app
/family-app/src-tauri/gen/
/family-app/src-tauri/target/
/family-app/ui/node_modules/
```

- [ ] **Step 2: Create the crate manifest, build script and window/security config**

`family-app/src-tauri/Cargo.toml`:

```toml
[package]
name = "family-app"
version = "0.1.0"
edition = "2021"
description = "Trellis family companion app (Tauri shell around family-app-core)"

[lib]
name = "family_app_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "=2.7.1", features = [] }

[dependencies]
family-app-core = { path = "../../tools/family-app-core" }
family-vault = { path = "../../programs/family-vault", features = ["cpi"] }
family-vault-client = { path = "../../tools/family-vault-client" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
solana-client = "=4.2.2"
solana-commitment-config = "=3.1.1"
solana-keypair = "3.0.1"
solana-pubkey = "=4.2.1"
solana-signature = "=3.4.1"
solana-signer = "3.0.0"
tauri = { version = "=2.12.2", features = [] }

[lints.rust]
unsafe_code = "deny"
dead_code = "deny"
```

`family-app/src-tauri/build.rs`:

```rust
fn main() {
    tauri_build::build()
}
```

`family-app/src-tauri/tauri.conf.json`:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Trellis Family",
  "version": "0.1.0",
  "identifier": "dev.trellis.family",
  "build": { "frontendDist": "../ui" },
  "app": {
    "withGlobalTauri": true,
    "windows": [
      { "title": "Trellis Family", "width": 480, "height": 820, "minWidth": 360, "minHeight": 600 }
    ],
    "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src ipc: http://ipc.localhost"
    }
  },
  "bundle": { "active": false }
}
```

The CSP has no `unsafe-inline`: the UI uses only external scripts and stylesheets, and `connect-src ipc:` is what Tauri's IPC needs.

- [ ] **Step 3: Create a placeholder icon and page**

Tauri refuses to build without `icons/icon.png` (32-bit RGBA). Generate a plain placeholder (real branding is out of scope):

```bash
mkdir -p family-app/src-tauri/icons family-app/ui
python3 - <<'EOF'
import zlib, struct
def png(w, h):
    raw = b''.join(b'\x00' + b'\x10\x80\x40\xff' * w for _ in range(h))
    def chunk(t, d):
        c = struct.pack('>I', len(d)) + t + d
        return c + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b''))
open('family-app/src-tauri/icons/icon.png', 'wb').write(png(32, 32))
EOF
```

Create `family-app/ui/index.html` (Task 6 replaces it):

```html
<!doctype html>
<html lang="ru"><head><meta charset="utf-8"><title>Trellis Family</title></head><body></body></html>
```

- [ ] **Step 4: Create the sources**

`family-app/src-tauri/src/main.rs`:

```rust
// Prevents an extra console window on Windows in release; do not remove.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    family_app_lib::run()
}
```

`family-app/src-tauri/src/config.rs`:

```rust
//! Network settings. The app talks to Devnet only (the program is an
//! unaudited prototype); `TRELLIS_RPC_URL` overrides the endpoint for
//! development against a private node.

use solana_pubkey::Pubkey;

pub const DEFAULT_RPC_URL: &str = "https://api.devnet.solana.com";

/// Circle's Devnet USDC, the token every Trellis vault holds.
pub const DEVNET_USDC_MINT: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";

pub const IDENTITY_FILE: &str = "identity.json";
pub const PENDING_FILE: &str = "pending-transaction.json";

/// What the Devnet faucet is asked for: enough for a few hundred transactions.
pub const AIRDROP_LAMPORTS: u64 = 100_000_000;

pub fn rpc_url() -> String {
    std::env::var("TRELLIS_RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_string())
}

pub fn program_id() -> Pubkey {
    family_vault_client::to_client(family_vault::ID)
}

pub fn usdc_mint() -> Pubkey {
    DEVNET_USDC_MINT
        .parse()
        .expect("DEVNET_USDC_MINT is a valid base58 pubkey")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_usdc_mint_constant_parses() {
        assert_eq!(usdc_mint().to_string(), DEVNET_USDC_MINT);
    }

    #[test]
    fn the_program_id_is_the_declared_family_vault_id() {
        assert_eq!(program_id().to_bytes(), family_vault::ID.to_bytes());
    }
}
```

`family-app/src-tauri/src/state.rs`:

```rust
//! Everything the commands share. Cheap to clone (the heavy parts are
//! behind `Arc`), because blocking RPC work runs on another thread and
//! needs its own handle.

use std::path::PathBuf;
use std::sync::Arc;

use solana_client::rpc_client::RpcClient;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

use crate::config::PENDING_FILE;

#[derive(Clone)]
pub struct AppState {
    pub identity: Arc<Keypair>,
    pub rpc: Arc<RpcClient>,
    pub program_id: Pubkey,
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn me(&self) -> Pubkey {
        self.identity.pubkey()
    }

    pub fn pending_path(&self) -> PathBuf {
        self.data_dir.join(PENDING_FILE)
    }
}
```

`family-app/src-tauri/src/lib.rs`:

```rust
//! Trellis Family: the family member's app. A thin Tauri shell around
//! `family-app-core`: `commands` is the IPC surface, `text`/`view` hold
//! every rule and sentence as pure, tested functions, and the `ui/` folder
//! only displays what these return.

// Removed in Task 4, once the commands use every item.
#![allow(dead_code)]

mod config;
mod state;

use std::sync::Arc;

use family_app_core::identity::generate_or_load_identity;
use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use tauri::Manager;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
            let identity = generate_or_load_identity(&data_dir.join(config::IDENTITY_FILE))
                .map_err(|e| e.to_string())?;
            // `confirmed`, so a screen refreshed right after a send shows it.
            let rpc =
                RpcClient::new_with_commitment(config::rpc_url(), CommitmentConfig::confirmed());
            app.manage(AppState {
                identity: Arc::new(identity),
                rpc: Arc::new(rpc),
                program_id: config::program_id(),
                data_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running the Trellis Family app");
}
```

- [ ] **Step 5: Build, test and lint**

Run: `cargo fmt -p family-app && cargo test -p family-app 2>&1 | grep "test result" | head -1 && cargo clippy -p family-app --all-targets -- -D warnings 2>&1 | tail -3 && cargo check --workspace 2>&1 | tail -1`
Expected: `test result: ok. 2 passed`, no clippy output, and the workspace check finishes. The first build compiles the whole Tauri tree and takes a few minutes.

Then prove no existing package was bumped: `git diff -- Cargo.lock | grep -E "^-" | grep -v "^---" | head`. Expected: no output (the lock file only gained packages). If a line appears, a pre-existing package changed version: stop and investigate before committing.

- [ ] **Step 6: Commit (including `Cargo.lock`)**

```bash
git add Cargo.toml Cargo.lock .gitignore family-app
git commit -m "feat(family-app): scaffold the Tauri crate with identity, config and shared state"
```

---

### Task 3: Pure logic: screen model, permissions and every sentence (`dto`, `text`, `view`)

**Files:**
- Create: `family-app/src-tauri/src/{dto.rs,text.rs,view.rs}`
- Modify: `family-app/src-tauri/src/lib.rs` (declare the three modules)

**Interfaces:**
- Consumes: `family_app_core::vault_view::{VaultView, Payee, PendingRequest, sanitize_text}` (`VaultView::remaining_today()`, `Payee::is_active(now)`), `family_app_core::history::HistoryEntryKind`, `family_app_core::pairing::PairingError`, `family_app_core::actions::SignatureOutcome`, `family_app_core::ActionError`, `family_vault_client::format_base_units`, `family_vault::errors::VaultError`.
- Produces (used by Task 4):
  - `dto::*`: `ErrorDto{kind,message,details}`, `AppInfoDto`, `WalletDto`, `VaultRefDto`, `PayeeDto`, `GuardianDto`, `PendingRequestDto`, `PermissionsDto`, `VaultDto`, `HistoryLineDto`, `ActionOkDto`, `PendingTxDto`, `ResolutionDto`
  - `text::{short_address, plural_ru, money, payee_status, history_line, friendly_rejection, error_dto, resolution_for, pairing_error_dto}`
  - `view::{is_owner, is_guardian, permissions, plan_send, SendPlan, vault_dto}` where `SendPlan` is `Direct | Request | InsufficientFunds`

This is the heart of the app's correctness and it has no I/O: 26 unit tests (the 2 from Task 2 plus 24 here) run in milliseconds. `permissions` is the single place that says who may do what (deviation 5).

- [ ] **Step 1: Create `dto.rs`, `text.rs` and `view.rs`**

`family-app/src-tauri/src/dto.rs`:

```rust
//! What crosses the IPC boundary. Plain, serializable, already phrased in
//! the app's language: the JavaScript side only displays these and never
//! does money math (u64 amounts do not fit a JS number, so they travel as
//! strings that Rust formatted).

use serde::Serialize;

/// A failed command. `kind` is one of `no_connection`, `rejected`,
/// `unconfirmed`, `blocked`, `invalid`, `internal`.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct ErrorDto {
    pub kind: &'static str,
    /// Plain-language text for the screen.
    pub message: String,
    /// Technical text for a "details" fold-out; never the only explanation.
    pub details: Option<String>,
}

impl ErrorDto {
    pub fn new(kind: &'static str, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct AppInfoDto {
    pub address: String,
    pub address_short: String,
    pub network: &'static str,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct WalletDto {
    /// SOL for fees, already formatted ("0.1").
    pub sol: String,
    pub low_sol: bool,
}

/// An entry for the vault picker; cheap to build (no balance lookups).
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct VaultRefDto {
    pub address: String,
    pub address_short: String,
    /// `true` when this key owns the vault (created it).
    pub mine: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct PayeeDto {
    pub name: String,
    pub address: String,
    pub address_short: String,
    pub active: bool,
    /// "Можно отправлять" or "Можно будет через 5 часов".
    pub status: String,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct GuardianDto {
    pub address: String,
    pub address_short: String,
    pub is_me: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct PendingRequestDto {
    pub nonce: u64,
    pub amount: String,
    pub to: String,
    pub to_short: String,
    pub requested_at: i64,
}

/// What this key may do on this vault, decided once in Rust. The screens
/// only show the buttons that are `true`; the commands check again.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionsDto {
    pub deposit: bool,
    pub send: bool,
    pub add_payee: bool,
    pub add_guardian: bool,
    pub approve: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct VaultDto {
    pub address: String,
    pub address_short: String,
    /// "Семейный счёт" or "Личный счёт".
    pub title: String,
    pub is_pool: bool,
    pub is_owner: bool,
    pub is_guardian: bool,
    pub balance: String,
    pub decimals: u32,
    /// `None` when the vault has no daily limit.
    pub daily_limit: Option<String>,
    pub remaining_today: Option<String>,
    pub payees: Vec<PayeeDto>,
    pub guardians: Vec<GuardianDto>,
    pub pending: Vec<PendingRequestDto>,
    pub permissions: PermissionsDto,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct HistoryLineDto {
    pub signature: String,
    pub text: String,
    /// Unix seconds, `None` if the node does not know the block time.
    pub time: Option<i64>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct ActionOkDto {
    pub signature: String,
    /// What happened, in words ("Деньги отправлены", "Запрос отправлен ...").
    pub message: String,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct PendingTxDto {
    pub what: String,
    pub signature: String,
}

/// `status`: `none`, `waiting`, `confirmed`, `failed`, `expired`.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct ResolutionDto {
    pub status: &'static str,
    pub message: String,
}
```

`family-app/src-tauri/src/text.rs`:

```rust
//! Every sentence the app shows, and the pure functions that turn core
//! values into them. Plain bank language: no "vault", "PDA", "mint" or
//! "approver" anywhere a person reads it.

use family_app_core::history::HistoryEntryKind;
use family_app_core::pairing::PairingError;
use family_app_core::vault_view::sanitize_text;
use family_app_core::ActionError;
use family_vault::errors::VaultError;
use family_vault_client::format_base_units;
use solana_pubkey::Pubkey;

use family_app_core::actions::SignatureOutcome;

use crate::dto::{ErrorDto, ResolutionDto};

pub fn short_address(address: &Pubkey) -> String {
    let full = address.to_string();
    format!("{}…{}", &full[..4], &full[full.len() - 4..])
}

/// "1 час", "2 часа", "5 часов", "11 часов", "21 час".
pub fn plural_ru(n: u64, one: &str, few: &str, many: &str) -> String {
    let word = match (n % 100, n % 10) {
        (11..=14, _) => many,
        (_, 1) => one,
        (_, 2..=4) => few,
        _ => many,
    };
    format!("{n} {word}")
}

pub fn money(amount: u64, decimals: u32) -> String {
    format!("{} USDC", format_base_units(amount, decimals))
}

pub fn payee_status(active_after: i64, now: i64) -> String {
    if now >= active_after {
        return "Можно отправлять".to_string();
    }
    let secs = (active_after - now) as u64;
    if secs < 3600 {
        return "Можно будет меньше чем через час".to_string();
    }
    let hours = secs.div_ceil(3600);
    format!(
        "Можно будет через {}",
        plural_ru(hours, "час", "часа", "часов")
    )
}

pub fn history_line(kind: &HistoryEntryKind, decimals: u32) -> String {
    // Names in history come straight from the chain: clean them first.
    let quote = |name: &str| format!("«{}»", sanitize_text(name));
    match kind {
        HistoryEntryKind::Deposit { amount } => {
            format!("Пополнение на {}", money(*amount, decimals))
        }
        HistoryEntryKind::Withdraw { amount } => format!("Отправлено {}", money(*amount, decimals)),
        HistoryEntryKind::AddBeneficiary { name } => format!("Добавлен получатель {}", quote(name)),
        HistoryEntryKind::AddBeneficiaryUrgent { name } => {
            format!("Срочно добавлен получатель {}", quote(name))
        }
        HistoryEntryKind::AddTrustedApprover => "Добавлено доверенное лицо".to_string(),
        HistoryEntryKind::RequestWithdrawal { amount, .. } => {
            format!("Запрос на перевод {}", money(*amount, decimals))
        }
        HistoryEntryKind::ApproveWithdrawal => "Перевод одобрен".to_string(),
        HistoryEntryKind::InitializeVault => "Счёт открыт".to_string(),
        HistoryEntryKind::InitializeApprovers => "Включены доверенные лица".to_string(),
        HistoryEntryKind::Unknown => "Операция".to_string(),
    }
}

/// The Anchor error code inside a node's error text, e.g.
/// "custom program error: 0x1775" -> 6005.
fn custom_error_code(text: &str) -> Option<u32> {
    let marker = "custom program error: 0x";
    let start = text.find(marker)? + marker.len();
    let hex: String = text[start..]
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect();
    u32::from_str_radix(&hex, 16).ok()
}

/// A sentence for a rejection we can explain, or `None`.
pub fn friendly_rejection(text: &str) -> Option<&'static str> {
    let code = custom_error_code(text)?;
    let is = |e: VaultError| code == u32::from(e);
    Some(if is(VaultError::BeneficiaryCooldownActive) {
        "Этого получателя ещё нельзя использовать: после добавления нужно подождать 48 часов."
    } else if is(VaultError::UnknownBeneficiary) {
        "Этого человека нет в списке получателей."
    } else if is(VaultError::DailyLimitExceeded) {
        "Это больше, чем можно отправить за день. Нужен запрос на перевод."
    } else if is(VaultError::AmountDoesNotExceedDailyLimit) {
        "Эту сумму можно отправить сразу, без запроса."
    } else if is(VaultError::NotVaultOwner) {
        "Это может сделать только владелец счёта."
    } else if is(VaultError::NotTrustedApprover) {
        "Одобрять переводы могут только доверенные лица."
    } else if is(VaultError::BeneficiaryListFull) {
        "В списке получателей больше нет места."
    } else if is(VaultError::BeneficiaryAlreadyExists) {
        "Этот получатель уже есть в списке."
    } else if is(VaultError::ApproverListFull) {
        "В списке доверенных лиц больше нет места."
    } else if is(VaultError::ApproverAlreadyExists) {
        "Это доверенное лицо уже добавлено."
    } else if is(VaultError::CannotApproveOwnWithdrawals) {
        "Владелец счёта не может быть своим же доверенным лицом."
    } else if is(VaultError::ExistingApproverRequired) {
        "Чтобы добавить ещё одно доверенное лицо, нужна подпись уже добавленного."
    } else if is(VaultError::RequestAlreadyExecuted) {
        "Этот запрос уже выполнен."
    } else if is(VaultError::ZeroAmount) {
        "Сумма должна быть больше нуля."
    } else {
        return None;
    })
}

pub fn error_dto(error: &ActionError) -> ErrorDto {
    match error {
        ActionError::NoConnection => ErrorDto::new(
            "no_connection",
            "Нет связи. Проверьте интернет и попробуйте ещё раз.",
        ),
        ActionError::Rejected(text) => {
            let message = friendly_rejection(text).unwrap_or("Не получилось. Ничего не списано.");
            ErrorDto::new("rejected", message).with_details(text.clone())
        }
        ActionError::Unconfirmed { signature, .. } => ErrorDto::new(
            "unconfirmed",
            "Операция отправлена, но мы пока не знаем, дошла ли она. \
             Не повторяйте её: нажмите «Проверить».",
        )
        .with_details(signature.to_string()),
    }
}

/// What to tell the user once an unconfirmed operation has been looked up.
/// `Confirmed`, `Failed` and `Expired` settle it (the record may be deleted);
/// `StillPossible` does not.
pub fn resolution_for(outcome: &SignatureOutcome) -> ResolutionDto {
    match outcome {
        SignatureOutcome::Confirmed => ResolutionDto {
            status: "confirmed",
            message: "Операция прошла.".to_string(),
        },
        SignatureOutcome::Failed(_) => ResolutionDto {
            status: "failed",
            message: "Операция не выполнилась, деньги не списаны. Можно попробовать ещё раз."
                .to_string(),
        },
        SignatureOutcome::Expired => ResolutionDto {
            status: "expired",
            message: "Операция так и не дошла, деньги не списаны. Можно попробовать ещё раз."
                .to_string(),
        },
        SignatureOutcome::StillPossible => ResolutionDto {
            status: "waiting",
            message: "Пока неизвестно, дошла ли операция. Подождите минуту и проверьте ещё раз."
                .to_string(),
        },
    }
}

pub fn pairing_error_dto(error: &PairingError) -> ErrorDto {
    let (kind, message): (&'static str, &str) = match error {
        PairingError::BadQrText => (
            "invalid",
            "Это не похоже на код с экрана устройства. Наведите камеру на QR-код ещё раз.",
        ),
        PairingError::BadPin => ("invalid", "Код состоит ровно из 6 цифр."),
        PairingError::BadName(_) => (
            "invalid",
            "Имя нужно написать короче и без служебных символов.",
        ),
        PairingError::NoAdapter => (
            "rejected",
            "Не найден Bluetooth. Включите Bluetooth и попробуйте ещё раз.",
        ),
        PairingError::KioskNotFound => (
            "rejected",
            "Устройство не найдено. Подойдите к нему ближе и убедитесь, что на экране показан QR-код.",
        ),
        PairingError::ConnectFailed(_) => (
            "rejected",
            "Не удалось соединиться с устройством. Подойдите ближе и попробуйте ещё раз.",
        ),
        PairingError::WriteRejected(_) => (
            "rejected",
            "Устройство не приняло код. Проверьте код: на экране мог появиться новый.",
        ),
        PairingError::Ble(_) => ("rejected", "Ошибка Bluetooth. Попробуйте ещё раз."),
    };
    let details = match error {
        PairingError::BadName(d)
        | PairingError::ConnectFailed(d)
        | PairingError::WriteRejected(d)
        | PairingError::Ble(d) => Some(d.clone()),
        _ => None,
    };
    let dto = ErrorDto::new(kind, message);
    match details {
        Some(d) => dto.with_details(d),
        None => dto,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> Pubkey {
        Pubkey::new_from_array([byte; 32])
    }

    #[test]
    fn plural_ru_follows_the_russian_rules() {
        let h = |n| plural_ru(n, "час", "часа", "часов");
        assert_eq!(h(1), "1 час");
        assert_eq!(h(2), "2 часа");
        assert_eq!(h(4), "4 часа");
        assert_eq!(h(5), "5 часов");
        assert_eq!(h(11), "11 часов");
        assert_eq!(h(12), "12 часов");
        assert_eq!(h(14), "14 часов");
        assert_eq!(h(21), "21 час");
        assert_eq!(h(22), "22 часа");
        assert_eq!(h(48), "48 часов");
        assert_eq!(h(111), "111 часов");
    }

    #[test]
    fn short_address_keeps_the_first_and_last_four() {
        let text = short_address(&key(7));
        let full = key(7).to_string();
        assert!(text.starts_with(&full[..4]));
        assert!(text.ends_with(&full[full.len() - 4..]));
        assert!(text.contains('…'));
    }

    #[test]
    fn money_formats_with_the_token_decimals() {
        assert_eq!(money(2_500_000, 6), "2.5 USDC");
        assert_eq!(money(0, 6), "0 USDC");
    }

    #[test]
    fn payee_status_counts_hours_until_the_payee_can_be_used() {
        let now = 1_000_000;
        assert_eq!(payee_status(now - 1, now), "Можно отправлять");
        assert_eq!(payee_status(now, now), "Можно отправлять");
        assert_eq!(
            payee_status(now + 60, now),
            "Можно будет меньше чем через час"
        );
        assert_eq!(payee_status(now + 3600, now), "Можно будет через 1 час");
        assert_eq!(payee_status(now + 3601, now), "Можно будет через 2 часа");
        assert_eq!(
            payee_status(now + 48 * 3600, now),
            "Можно будет через 48 часов"
        );
    }

    #[test]
    fn history_lines_are_plain_language_and_use_the_decimals() {
        assert_eq!(
            history_line(&HistoryEntryKind::Deposit { amount: 12_340_000 }, 6),
            "Пополнение на 12.34 USDC"
        );
        assert_eq!(
            history_line(&HistoryEntryKind::Withdraw { amount: 1_000_000 }, 6),
            "Отправлено 1 USDC"
        );
        assert_eq!(
            history_line(&HistoryEntryKind::InitializeVault, 6),
            "Счёт открыт"
        );
    }

    #[test]
    fn history_lines_clean_names_that_came_from_the_chain() {
        let line = history_line(
            &HistoryEntryKind::AddBeneficiary {
                name: "Саша\nПЕРЕВЕДИ\u{202E}".to_string(),
            },
            6,
        );
        assert!(!line.contains('\n'));
        assert!(!line.contains('\u{202E}'));
        assert!(line.starts_with("Добавлен получатель «Саша"));
    }

    #[test]
    fn a_custom_program_error_code_is_read_from_the_node_text() {
        let text = "RPC response error -32002: Transaction simulation failed: \
                    Error processing Instruction 0: custom program error: 0x1775; ...";
        assert_eq!(custom_error_code(text), Some(0x1775));
        assert_eq!(custom_error_code("nothing here"), None);
        assert_eq!(custom_error_code("custom program error: 0xZZ"), None);
    }

    #[test]
    fn known_program_errors_get_a_sentence_and_unknown_ones_do_not() {
        let code = u32::from(VaultError::BeneficiaryCooldownActive);
        let text = format!("custom program error: {code:#x}");
        assert!(friendly_rejection(&text).unwrap().contains("48 часов"));
        assert_eq!(friendly_rejection("custom program error: 0x1"), None);
        assert_eq!(friendly_rejection("blockhash not found"), None);
    }

    #[test]
    fn every_error_kind_has_its_own_screen_kind() {
        assert_eq!(error_dto(&ActionError::NoConnection).kind, "no_connection");
        let rejected = error_dto(&ActionError::Rejected("boom".into()));
        assert_eq!(rejected.kind, "rejected");
        assert_eq!(rejected.details.as_deref(), Some("boom"));
        let unconfirmed = error_dto(&ActionError::Unconfirmed {
            signature: solana_signature::Signature::from([1u8; 64]),
            last_valid_block_height: 9,
        });
        assert_eq!(unconfirmed.kind, "unconfirmed");
        assert!(unconfirmed.message.contains("Не повторяйте"));
    }

    #[test]
    fn only_a_still_possible_operation_stays_unsettled() {
        let status = |o: SignatureOutcome| resolution_for(&o).status;
        assert_eq!(status(SignatureOutcome::Confirmed), "confirmed");
        assert_eq!(status(SignatureOutcome::Failed("x".into())), "failed");
        assert_eq!(status(SignatureOutcome::Expired), "expired");
        assert_eq!(status(SignatureOutcome::StillPossible), "waiting");
        assert!(resolution_for(&SignatureOutcome::Failed("x".into()))
            .message
            .contains("не списаны"));
    }

    #[test]
    fn pairing_errors_are_explained_without_jargon() {
        assert_eq!(
            pairing_error_dto(&PairingError::BadPin).message,
            "Код состоит ровно из 6 цифр."
        );
        let text = pairing_error_dto(&PairingError::KioskNotFound).message;
        assert!(text.contains("ближе"));
        let rejected = pairing_error_dto(&PairingError::WriteRejected("att 0x0e".into()));
        assert_eq!(rejected.details.as_deref(), Some("att 0x0e"));
    }
}
```

`family-app/src-tauri/src/view.rs`:

```rust
//! Turns a core `VaultView` into the screen model, and decides what this
//! key is allowed to do with that vault. Pure: no RPC, no clock (the caller
//! passes `now`), so every rule is unit-tested below.

use family_app_core::vault_view::VaultView;
use solana_pubkey::Pubkey;

use crate::dto::{GuardianDto, PayeeDto, PendingRequestDto, PermissionsDto, VaultDto};
use crate::text::{money, payee_status, short_address};

pub fn is_owner(view: &VaultView, me: &Pubkey) -> bool {
    view.owner == *me
}

pub fn is_guardian(view: &VaultView, me: &Pubkey) -> bool {
    view.guardians.contains(me)
}

/// The rules, in one place:
/// - anyone may deposit;
/// - only the owner sends, adds payees and adds guardians (the program
///   checks the owner's signature);
/// - sending needs at least one payee who is already past the 48-hour wait;
/// - on a personal vault that already has a guardian, adding another needs
///   that guardian to co-sign, which a single device cannot do, so it is
///   not offered;
/// - only a guardian approves, and only when something is waiting.
pub fn permissions(view: &VaultView, me: &Pubkey, now: i64) -> PermissionsDto {
    let owner = is_owner(view, me);
    PermissionsDto {
        deposit: true,
        send: owner && view.payees.iter().any(|p| p.is_active(now)),
        add_payee: owner,
        add_guardian: owner && (view.is_pool || view.guardians.is_empty()),
        approve: is_guardian(view, me) && !view.pending.is_empty(),
    }
}

/// How a send of `amount` base units must be done on this vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendPlan {
    /// Within the allowance: send straight away.
    Direct,
    /// Above the daily limit of a pool: file a request a guardian approves.
    Request,
    /// The vault does not hold that much.
    InsufficientFunds,
}

pub fn plan_send(view: &VaultView, amount: u64) -> SendPlan {
    if amount > view.balance {
        SendPlan::InsufficientFunds
    } else if view.is_pool && view.remaining_today().is_some_and(|left| amount > left) {
        SendPlan::Request
    } else {
        SendPlan::Direct
    }
}

pub fn vault_dto(view: &VaultView, me: &Pubkey, now: i64) -> VaultDto {
    let decimals = view.decimals;
    VaultDto {
        address: view.vault.to_string(),
        address_short: short_address(&view.vault),
        title: if view.is_pool {
            "Семейный счёт".to_string()
        } else {
            "Личный счёт".to_string()
        },
        is_pool: view.is_pool,
        is_owner: is_owner(view, me),
        is_guardian: is_guardian(view, me),
        balance: money(view.balance, decimals),
        decimals,
        daily_limit: view.daily_limit.map(|v| money(v, decimals)),
        remaining_today: view.remaining_today().map(|v| money(v, decimals)),
        payees: view
            .payees
            .iter()
            .map(|p| PayeeDto {
                name: p.name.clone(),
                address: p.address.to_string(),
                address_short: short_address(&p.address),
                active: p.is_active(now),
                status: payee_status(p.active_after, now),
            })
            .collect(),
        guardians: view
            .guardians
            .iter()
            .map(|g| GuardianDto {
                address: g.to_string(),
                address_short: short_address(g),
                is_me: g == me,
            })
            .collect(),
        pending: view
            .pending
            .iter()
            .map(|r| PendingRequestDto {
                nonce: r.nonce,
                amount: money(r.amount, decimals),
                to: r.to.to_string(),
                to_short: short_address(&r.to),
                requested_at: r.requested_at,
            })
            .collect(),
        permissions: permissions(view, me, now),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use family_app_core::vault_view::{Payee, PendingRequest};

    const NOW: i64 = 1_000_000;

    fn key(byte: u8) -> Pubkey {
        Pubkey::new_from_array([byte; 32])
    }

    fn payee(byte: u8, active_after: i64) -> Payee {
        Payee {
            name: format!("Получатель {byte}"),
            address: key(byte),
            active_after,
        }
    }

    fn view(owner: u8, is_pool: bool) -> VaultView {
        VaultView {
            vault: key(100),
            owner: key(owner),
            is_pool,
            daily_limit: if is_pool { Some(100_000_000) } else { None },
            spent_today: 30_000_000,
            mint: key(101),
            decimals: 6,
            balance: 250_500_000,
            payees: vec![payee(10, NOW - 1), payee(11, NOW + 3600)],
            guardians: vec![key(2)],
            pending: vec![],
            next_free_nonce: 0,
        }
    }

    fn request() -> PendingRequest {
        PendingRequest {
            nonce: 3,
            address: key(200),
            to: key(10),
            amount: 5_000_000,
            requested_at: NOW - 60,
        }
    }

    #[test]
    fn the_owner_can_send_add_payees_and_guardians_but_not_approve() {
        let me = key(1);
        let p = permissions(&view(1, true), &me, NOW);
        assert!(p.deposit && p.send && p.add_payee && p.add_guardian);
        assert!(!p.approve);
    }

    #[test]
    fn a_guardian_who_is_not_the_owner_can_only_deposit_and_approve() {
        let me = key(2);
        let mut v = view(1, true);
        v.pending.push(request());
        let p = permissions(&v, &me, NOW);
        assert!(p.deposit && p.approve);
        assert!(!p.send && !p.add_payee && !p.add_guardian);
    }

    #[test]
    fn approving_needs_something_to_approve() {
        let p = permissions(&view(1, true), &key(2), NOW);
        assert!(!p.approve);
    }

    #[test]
    fn someone_with_no_role_can_only_deposit() {
        let mut v = view(1, true);
        v.pending.push(request());
        let p = permissions(&v, &key(9), NOW);
        assert!(p.deposit);
        assert!(!p.send && !p.add_payee && !p.add_guardian && !p.approve);
    }

    #[test]
    fn sending_needs_a_payee_who_is_past_the_wait() {
        let me = key(1);
        let mut v = view(1, true);
        v.payees = vec![payee(11, NOW + 3600)];
        assert!(!permissions(&v, &me, NOW).send);
        assert!(permissions(&v, &me, NOW + 3600).send);
        v.payees.clear();
        assert!(!permissions(&v, &me, NOW).send);
    }

    #[test]
    fn a_second_guardian_on_a_personal_vault_needs_a_cosigner_so_it_is_not_offered() {
        let me = key(1);
        // Personal vault that already has a guardian: not offered.
        assert!(!permissions(&view(1, false), &me, NOW).add_guardian);
        // Personal vault with no guardian yet: the owner alone may add the first.
        let mut v = view(1, false);
        v.guardians.clear();
        assert!(permissions(&v, &me, NOW).add_guardian);
        // A pool never needs a cosigner.
        assert!(permissions(&view(1, true), &me, NOW).add_guardian);
    }

    #[test]
    fn a_send_within_the_allowance_goes_straight_through() {
        // 100 limit, 30 spent -> 70 left; balance 250.5
        assert_eq!(plan_send(&view(1, true), 70_000_000), SendPlan::Direct);
        assert_eq!(plan_send(&view(1, true), 1), SendPlan::Direct);
    }

    #[test]
    fn a_send_above_the_daily_limit_of_a_pool_becomes_a_request() {
        assert_eq!(plan_send(&view(1, true), 70_000_001), SendPlan::Request);
        assert_eq!(plan_send(&view(1, true), 200_000_000), SendPlan::Request);
    }

    #[test]
    fn a_personal_vault_never_needs_a_request() {
        assert_eq!(plan_send(&view(1, false), 250_000_000), SendPlan::Direct);
    }

    #[test]
    fn asking_for_more_than_the_balance_is_refused_before_anything_else() {
        assert_eq!(
            plan_send(&view(1, true), 250_500_001),
            SendPlan::InsufficientFunds
        );
        assert_eq!(
            plan_send(&view(1, false), 250_500_001),
            SendPlan::InsufficientFunds
        );
        assert_eq!(plan_send(&view(1, false), 250_500_000), SendPlan::Direct);
    }

    #[test]
    fn the_screen_model_uses_plain_words_and_formatted_money() {
        let mut v = view(1, true);
        v.pending.push(request());
        let dto = vault_dto(&v, &key(1), NOW);
        assert_eq!(dto.title, "Семейный счёт");
        assert_eq!(dto.balance, "250.5 USDC");
        assert_eq!(dto.daily_limit.as_deref(), Some("100 USDC"));
        assert_eq!(dto.remaining_today.as_deref(), Some("70 USDC"));
        assert!(dto.is_owner && !dto.is_guardian);
        assert_eq!(dto.payees.len(), 2);
        assert!(dto.payees[0].active);
        assert_eq!(dto.payees[0].status, "Можно отправлять");
        assert!(!dto.payees[1].active);
        assert_eq!(dto.payees[1].status, "Можно будет через 1 час");
        assert_eq!(dto.pending[0].amount, "5 USDC");
        assert_eq!(dto.pending[0].nonce, 3);
        assert!(dto.guardians[0].address_short.contains('…'));
    }

    #[test]
    fn a_personal_vault_has_no_limit_and_is_titled_personal() {
        let dto = vault_dto(&view(1, false), &key(1), NOW);
        assert_eq!(dto.title, "Личный счёт");
        assert_eq!(dto.daily_limit, None);
        assert_eq!(dto.remaining_today, None);
    }

    #[test]
    fn the_guardian_flag_marks_me_in_the_guardian_list() {
        let dto = vault_dto(&view(1, true), &key(2), NOW);
        assert!(dto.is_guardian);
        assert!(dto.guardians[0].is_me);
    }
}
```

- [ ] **Step 2: Declare the modules in `lib.rs`**

In `family-app/src-tauri/src/lib.rs` change

```rust
mod config;
mod state;
```

to

```rust
mod config;
mod dto;
mod state;
mod text;
mod view;
```

- [ ] **Step 3: Test and lint**

Run: `cargo fmt -p family-app && cargo test -p family-app 2>&1 | grep "test result" | head -1 && cargo clippy -p family-app --all-targets -- -D warnings 2>&1 | tail -3`
Expected: `test result: ok. 26 passed`, no clippy output.

- [ ] **Step 4: Commit**

```bash
git add family-app
git commit -m "feat(family-app): pure screen model, permissions and Russian text with tests"
```

---

### Task 4: Pending-operation record and the IPC commands

**Files:**
- Create: `family-app/src-tauri/src/pending.rs`, `family-app/src-tauri/src/commands/{mod.rs,read.rs,actions.rs,pairing.rs}`
- Replace: `family-app/src-tauri/src/lib.rs` (final version: all modules, all handlers, no `allow(dead_code)`)

**Interfaces:**
- Consumes: everything from Tasks 1-3, plus `family_app_core::{actions, discovery, history, wallet, pairing, vault_view}`.
- Produces (the contract Tasks 6-7 call from JavaScript; Tauri 2 passes JavaScript arguments in camelCase, Rust `qr_text` is `qrText`, `daily_limit` is `dailyLimit`):

| command | arguments | returns |
|---|---|---|
| `app_info` | | `AppInfoDto` |
| `wallet` | | `WalletDto` |
| `list_vaults` | | `VaultRefDto[]` (owned first) |
| `load_vault` | `vault` | `VaultDto` |
| `load_history` | `vault`, `decimals` | `HistoryLineDto[]` |
| `pending_status` | | `PendingTxDto \| null` |
| `check_pending` | | `ResolutionDto` |
| `deposit` | `vault`, `amount` (text) | `ActionOkDto` |
| `send_money` | `vault`, `recipient`, `amount` (text) | `ActionOkDto` (sends, or files a request above the daily limit) |
| `add_payee` | `vault`, `name`, `address` | `ActionOkDto` |
| `add_guardian` | `vault`, `address` | `ActionOkDto` |
| `approve_request` | `vault`, `nonce` | `ActionOkDto` |
| `open_family_account` | `dailyLimit` (text) | `VaultRefDto` |
| `get_test_sol` | | `null` |
| `pair_with_qr` | `qrText`, `name` | `null` |
| `pair_with_pin` | `pin`, `name` | `null` |

Every command rejects with an `ErrorDto`. The money commands share one rule set: refuse while an earlier operation is unresolved, re-read the vault and re-check `view::permissions` (the screen may be stale), parse amounts exactly, and record an `Unconfirmed` outcome in `pending.rs` so the rule survives a restart.

- [ ] **Step 1: Create `pending.rs` (with its 5 tests)**

`family-app/src-tauri/src/pending.rs`:

```rust
//! The one operation that was sent but whose outcome is not known yet.
//!
//! While this file exists, every command that moves money refuses to run
//! (see `commands::ensure_no_pending`): retrying with a fresh transaction
//! could pay twice. The file survives closing the app, so the rule holds
//! across restarts; it is removed only once the operation is settled.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PendingTx {
    pub signature: String,
    pub last_valid_block_height: u64,
    /// What the user was doing, in words ("Отправка 5 USDC").
    pub what: String,
}

pub fn load(path: &Path) -> Result<Option<PendingTx>, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| format!("could not read {}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("could not read {}: {e}", path.display())),
    }
}

/// Refuses to replace an existing record: an unresolved operation must
/// never be forgotten because a second one was started.
pub fn save(path: &Path, pending: &PendingTx) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(pending).map_err(|e| e.to_string())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("could not save {}: {e}", path.display()))?;
    file.write_all(&json)
        .and_then(|()| file.sync_all())
        .map_err(|e| format!("could not save {}: {e}", path.display()))
}

pub fn clear(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("could not remove {}: {e}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        std::env::temp_dir().join(format!("family-app-pending-{name}-{nanos}.json"))
    }

    fn sample() -> PendingTx {
        PendingTx {
            signature: "5j7s".to_string(),
            last_valid_block_height: 123_456,
            what: "Отправка 5 USDC".to_string(),
        }
    }

    #[test]
    fn nothing_saved_means_nothing_pending() {
        assert_eq!(load(&temp_path("none")).unwrap(), None);
    }

    #[test]
    fn a_saved_record_loads_back_unchanged() {
        let path = temp_path("roundtrip");
        save(&path, &sample()).unwrap();
        assert_eq!(load(&path).unwrap(), Some(sample()));
        clear(&path).unwrap();
        assert_eq!(load(&path).unwrap(), None);
    }

    #[test]
    fn a_second_save_cannot_overwrite_an_unresolved_one() {
        let path = temp_path("noclobber");
        save(&path, &sample()).unwrap();
        let other = PendingTx {
            signature: "other".to_string(),
            ..sample()
        };
        assert!(save(&path, &other).is_err());
        assert_eq!(load(&path).unwrap(), Some(sample()));
        clear(&path).unwrap();
    }

    #[test]
    fn a_corrupt_record_is_an_error_not_silently_ignored() {
        let path = temp_path("corrupt");
        std::fs::write(&path, b"not json").unwrap();
        assert!(load(&path).is_err());
        clear(&path).unwrap();
    }

    #[test]
    fn clearing_twice_is_fine() {
        let path = temp_path("twice");
        clear(&path).unwrap();
        clear(&path).unwrap();
    }
}
```

- [ ] **Step 2: Create the commands**

`family-app/src-tauri/src/commands/mod.rs`:

```rust
//! The IPC surface. Each command is thin: check the rules (permissions,
//! pending operation), call one `family-app-core` function on a blocking
//! thread, and phrase the outcome. No command does money math in
//! JavaScript terms: amounts arrive as text and are parsed here, exactly.

pub mod actions;
pub mod pairing;
pub mod read;

use family_app_core::vault_view::{fetch_vault_view, VaultView};
use family_app_core::ActionError;
use solana_pubkey::Pubkey;
use solana_signature::Signature;

use crate::dto::{ActionOkDto, ErrorDto};
use crate::pending::{self, PendingTx};
use crate::state::AppState;
use crate::text::error_dto;

impl From<ActionError> for ErrorDto {
    fn from(error: ActionError) -> Self {
        error_dto(&error)
    }
}

/// Runs blocking RPC/Bluetooth work off the async runtime.
pub async fn blocking<T, F>(work: F) -> Result<T, ErrorDto>
where
    F: FnOnce() -> Result<T, ErrorDto> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| {
            ErrorDto::new("internal", "Что-то пошло не так внутри приложения.")
                .with_details(e.to_string())
        })?
}

pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn parse_pubkey(text: &str, what: &str) -> Result<Pubkey, ErrorDto> {
    text.trim().parse().map_err(|_| {
        ErrorDto::new(
            "invalid",
            format!("{what}: это не похоже на адрес. Проверьте, что он скопирован целиком."),
        )
    })
}

/// Exact decimal parse; `text` is what the person typed ("25", "2.50").
pub fn parse_amount(text: &str, decimals: u32) -> Result<u64, ErrorDto> {
    family_vault_client::parse_decimal_amount(text, decimals).map_err(|e| {
        ErrorDto::new(
            "invalid",
            "Введите сумму числом, например 25 или 2.50 (не больше знаков после точки, чем у денег на счёте).",
        )
        .with_details(e)
    })
}

pub fn load_view(state: &AppState, vault: &str) -> Result<VaultView, ErrorDto> {
    let vault = parse_pubkey(vault, "Счёт")?;
    Ok(fetch_vault_view(&state.rpc, &state.program_id, &vault)?)
}

/// While an operation's outcome is unknown nothing else may move money.
pub fn ensure_no_pending(state: &AppState) -> Result<(), ErrorDto> {
    let waiting = pending::load(&state.pending_path()).map_err(|e| {
        ErrorDto::new("internal", "Не удалось прочитать записи приложения.").with_details(e)
    })?;
    match waiting {
        None => Ok(()),
        Some(p) => Err(ErrorDto::new(
            "blocked",
            format!(
                "Сначала нужно узнать, чем закончилась прошлая операция («{}»). Нажмите «Проверить».",
                p.what
            ),
        )),
    }
}

/// Remembers an unconfirmed operation so the rule above survives a restart.
pub fn record_unconfirmed(state: &AppState, what: &str, error: &ActionError) {
    if let ActionError::Unconfirmed {
        signature,
        last_valid_block_height,
    } = error
    {
        let record = PendingTx {
            signature: signature.to_string(),
            last_valid_block_height: *last_valid_block_height,
            what: what.to_string(),
        };
        // If this fails the user still sees the "unconfirmed" screen; the
        // worst case is that a restart forgets it, never a silent success.
        let _ = pending::save(&state.pending_path(), &record);
    }
}

/// Turns a core result into the command result, recording `Unconfirmed`.
pub fn finish(
    state: &AppState,
    what: &str,
    result: Result<Signature, ActionError>,
    ok_message: &str,
) -> Result<ActionOkDto, ErrorDto> {
    match result {
        Ok(signature) => Ok(ActionOkDto {
            signature: signature.to_string(),
            message: ok_message.to_string(),
        }),
        Err(error) => {
            record_unconfirmed(state, what, &error);
            Err(error.into())
        }
    }
}

pub fn not_allowed(text: &str) -> ErrorDto {
    ErrorDto::new("blocked", text)
}
```

`family-app/src-tauri/src/commands/read.rs`:

```rust
//! Read-only commands: who am I, what do I have, what happened.

use std::collections::BTreeSet;

use family_app_core::actions::{confirm_signature, SignatureOutcome};
use family_app_core::discovery::{discover_my_vaults, discover_owned_vaults};
use family_app_core::history::fetch_history;
use family_app_core::wallet::{sol_balance, LOW_SOL_LAMPORTS};
use family_vault_client::format_base_units;
use tauri::State;

use super::{blocking, load_view, now_unix, parse_pubkey};
use crate::dto::{
    AppInfoDto, ErrorDto, HistoryLineDto, PendingTxDto, ResolutionDto, VaultDto, VaultRefDto,
    WalletDto,
};
use crate::pending;
use crate::state::AppState;
use crate::text::{history_line, resolution_for, short_address};
use crate::view::vault_dto;

/// How many recent transactions the history screen asks the node for.
const HISTORY_LIMIT: usize = 10;

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfoDto {
    let me = state.me();
    AppInfoDto {
        address: me.to_string(),
        address_short: short_address(&me),
        network: "devnet",
    }
}

#[tauri::command]
pub async fn wallet(state: State<'_, AppState>) -> Result<WalletDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        let lamports = sol_balance(&st.rpc, &st.me())?;
        Ok(WalletDto {
            sol: format_base_units(lamports, 9),
            low_sol: lamports < LOW_SOL_LAMPORTS,
        })
    })
    .await
}

/// Every vault this key owns or takes part in, the ones it owns first.
#[tauri::command]
pub async fn list_vaults(state: State<'_, AppState>) -> Result<Vec<VaultRefDto>, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        let me = st.me();
        let owned = discover_owned_vaults(&st.rpc, &st.program_id, &me)?;
        let others: BTreeSet<_> = discover_my_vaults(&st.rpc, &st.program_id, &me)?
            .into_iter()
            .filter(|v| !owned.contains(v))
            .collect();
        let to_ref = |address: solana_pubkey::Pubkey, mine: bool| VaultRefDto {
            address: address.to_string(),
            address_short: short_address(&address),
            mine,
        };
        Ok(owned
            .into_iter()
            .map(|v| to_ref(v, true))
            .chain(others.into_iter().map(|v| to_ref(v, false)))
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn load_vault(state: State<'_, AppState>, vault: String) -> Result<VaultDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        let view = load_view(&st, &vault)?;
        Ok(vault_dto(&view, &st.me(), now_unix()))
    })
    .await
}

/// `decimals` only formats the amounts in the lines; it comes from the
/// `VaultDto` the screen already holds.
#[tauri::command]
pub async fn load_history(
    state: State<'_, AppState>,
    vault: String,
    decimals: u32,
) -> Result<Vec<HistoryLineDto>, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        let vault = parse_pubkey(&vault, "Счёт")?;
        let entries = fetch_history(&st.rpc, &st.program_id, &vault, HISTORY_LIMIT)?;
        Ok(entries
            .into_iter()
            .map(|e| HistoryLineDto {
                signature: e.signature,
                text: history_line(&e.kind, decimals),
                time: e.block_time,
            })
            .collect())
    })
    .await
}

/// The operation whose outcome is still unknown, if any.
#[tauri::command]
pub fn pending_status(state: State<'_, AppState>) -> Result<Option<PendingTxDto>, ErrorDto> {
    let record = pending::load(&state.pending_path()).map_err(|e| {
        ErrorDto::new("internal", "Не удалось прочитать записи приложения.").with_details(e)
    })?;
    Ok(record.map(|p| PendingTxDto {
        what: p.what,
        signature: p.signature,
    }))
}

/// Asks the node what became of the unconfirmed operation and, once that is
/// settled, forgets it so money commands work again.
#[tauri::command]
pub async fn check_pending(state: State<'_, AppState>) -> Result<ResolutionDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        let record = pending::load(&st.pending_path()).map_err(|e| {
            ErrorDto::new("internal", "Не удалось прочитать записи приложения.").with_details(e)
        })?;
        let Some(record) = record else {
            return Ok(ResolutionDto {
                status: "none",
                message: "Ничего не ожидает проверки.".to_string(),
            });
        };
        let signature = record.signature.parse().map_err(|_| {
            ErrorDto::new("internal", "Запись об операции повреждена.")
                .with_details(record.signature.clone())
        })?;
        let outcome = confirm_signature(&st.rpc, &signature, record.last_valid_block_height)?;
        if outcome != SignatureOutcome::StillPossible {
            pending::clear(&st.pending_path()).map_err(|e| {
                ErrorDto::new("internal", "Не удалось обновить записи приложения.").with_details(e)
            })?;
        }
        Ok(resolution_for(&outcome))
    })
    .await
}
```

`family-app/src-tauri/src/commands/actions.rs`:

```rust
//! Commands that change something on the chain. Every one of them:
//! 1. refuses while an earlier operation's outcome is unknown,
//! 2. re-reads the vault and re-checks the rule (the screen may be stale),
//! 3. parses amounts exactly,
//! 4. records an `Unconfirmed` result so the rule in (1) survives a restart.

use family_app_core::actions as core;
use family_app_core::wallet::request_devnet_airdrop;
use family_vault_client::fetch_mint_decimals;
use tauri::State;

use super::{
    blocking, ensure_no_pending, finish, load_view, not_allowed, parse_amount, parse_pubkey,
    record_unconfirmed,
};
use crate::config::{usdc_mint, AIRDROP_LAMPORTS};
use crate::dto::{ActionOkDto, ErrorDto, VaultRefDto};
use crate::state::AppState;
use crate::text::{money, short_address};
use crate::view::{permissions, plan_send, SendPlan};

#[tauri::command]
pub async fn deposit(
    state: State<'_, AppState>,
    vault: String,
    amount: String,
) -> Result<ActionOkDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        ensure_no_pending(&st)?;
        let view = load_view(&st, &vault)?;
        let amount = parse_amount(&amount, view.decimals)?;
        let result = core::deposit(&st.rpc, &st.program_id, &st.identity, &view.vault, amount);
        let what = format!("Пополнение на {}", money(amount, view.decimals));
        finish(&st, &what, result, "Деньги отправлены на счёт.")
    })
    .await
}

/// Sends to a payee, or, when a pool's daily limit would be exceeded, files
/// a request that a guardian must approve.
#[tauri::command]
pub async fn send_money(
    state: State<'_, AppState>,
    vault: String,
    recipient: String,
    amount: String,
) -> Result<ActionOkDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        ensure_no_pending(&st)?;
        let view = load_view(&st, &vault)?;
        let now = super::now_unix();
        if !permissions(&view, &st.me(), now).send {
            return Err(not_allowed(
                "Отправлять деньги с этого счёта может только его владелец, и нужен получатель, которого уже можно использовать.",
            ));
        }
        let recipient = parse_pubkey(&recipient, "Получатель")?;
        let payee_ok = view
            .payees
            .iter()
            .any(|p| p.address == recipient && p.is_active(now));
        if !payee_ok {
            return Err(ErrorDto::new(
                "invalid",
                "Выберите получателя из списка. Нового получателя можно использовать через 48 часов после добавления.",
            ));
        }
        let amount = parse_amount(&amount, view.decimals)?;
        let what = format!("Отправка {}", money(amount, view.decimals));
        match plan_send(&view, amount) {
            SendPlan::InsufficientFunds => Err(ErrorDto::new(
                "invalid",
                "На счёте не хватает денег для этой суммы.",
            )),
            SendPlan::Direct => {
                let result = core::send(
                    &st.rpc,
                    &st.program_id,
                    &st.identity,
                    &view.vault,
                    &recipient,
                    amount,
                );
                finish(&st, &what, result, "Деньги отправлены.")
            }
            SendPlan::Request => {
                let result = core::request_withdrawal(
                    &st.rpc,
                    &st.program_id,
                    &st.identity,
                    &view.vault,
                    &recipient,
                    amount,
                    view.next_free_nonce,
                );
                finish(
                    &st,
                    &what,
                    result,
                    "Это больше суммы, доступной за день. Запрос отправлен: деньги уйдут, когда доверенное лицо его одобрит.",
                )
            }
        }
    })
    .await
}

#[tauri::command]
pub async fn add_payee(
    state: State<'_, AppState>,
    vault: String,
    name: String,
    address: String,
) -> Result<ActionOkDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        ensure_no_pending(&st)?;
        let view = load_view(&st, &vault)?;
        if !permissions(&view, &st.me(), super::now_unix()).add_payee {
            return Err(not_allowed(
                "Добавлять получателей может только владелец счёта.",
            ));
        }
        let name = name.trim().to_string();
        if name.is_empty() || name.len() > 32 {
            return Err(ErrorDto::new(
                "invalid",
                "Имя должно быть непустым и не длиннее 32 байт (около 16 русских букв).",
            ));
        }
        let address = parse_pubkey(&address, "Получатель")?;
        let result = core::add_beneficiary(
            &st.rpc,
            &st.program_id,
            &st.identity,
            &view.vault,
            &name,
            &address,
        );
        let what = format!("Добавление получателя «{name}»");
        finish(
            &st,
            &what,
            result,
            "Получатель добавлен. Пользоваться им можно будет через 48 часов.",
        )
    })
    .await
}

#[tauri::command]
pub async fn add_guardian(
    state: State<'_, AppState>,
    vault: String,
    address: String,
) -> Result<ActionOkDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        ensure_no_pending(&st)?;
        let view = load_view(&st, &vault)?;
        if !permissions(&view, &st.me(), super::now_unix()).add_guardian {
            return Err(not_allowed(
                "Добавить доверенное лицо с этого устройства нельзя: для этого нужна подпись владельца счёта.",
            ));
        }
        let address = parse_pubkey(&address, "Доверенное лицо")?;
        let result = core::add_trusted_approver(
            &st.rpc,
            &st.program_id,
            &st.identity,
            &view.vault,
            &address,
            None,
        );
        finish(
            &st,
            "Добавление доверенного лица",
            result,
            "Доверенное лицо добавлено.",
        )
    })
    .await
}

#[tauri::command]
pub async fn approve_request(
    state: State<'_, AppState>,
    vault: String,
    nonce: u64,
) -> Result<ActionOkDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        ensure_no_pending(&st)?;
        let view = load_view(&st, &vault)?;
        if !permissions(&view, &st.me(), super::now_unix()).approve {
            return Err(not_allowed(
                "Одобрять переводы могут только доверенные лица.",
            ));
        }
        let request = view
            .pending
            .iter()
            .find(|r| r.nonce == nonce)
            .ok_or_else(|| ErrorDto::new("invalid", "Этого запроса уже нет в списке."))?;
        let what = format!(
            "Одобрение перевода на {}",
            money(request.amount, view.decimals)
        );
        let result =
            core::approve_withdrawal(&st.rpc, &st.program_id, &st.identity, &view.vault, nonce);
        finish(&st, &what, result, "Перевод одобрен.")
    })
    .await
}

/// Opens this key's family account (a pool vault) with a daily limit.
#[tauri::command]
pub async fn open_family_account(
    state: State<'_, AppState>,
    daily_limit: String,
) -> Result<VaultRefDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        ensure_no_pending(&st)?;
        let mint = usdc_mint();
        let decimals = fetch_mint_decimals(&st.rpc, &mint)
            .map_err(|e| ErrorDto::from(family_app_core::ActionError::Rejected(e)))?;
        let limit = parse_amount(&daily_limit, decimals)?;
        match core::open_pool_vault(&st.rpc, &st.program_id, &st.identity, &mint, limit) {
            Ok(vault) => Ok(VaultRefDto {
                address: vault.to_string(),
                address_short: short_address(&vault),
                mine: true,
            }),
            Err(error) => {
                record_unconfirmed(&st, "Открытие семейного счёта", &error);
                Err(error.into())
            }
        }
    })
    .await
}

/// Devnet only: asks the public faucet for SOL to pay transaction fees.
#[tauri::command]
pub async fn get_test_sol(state: State<'_, AppState>) -> Result<(), ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        request_devnet_airdrop(&st.rpc, &st.me(), AIRDROP_LAMPORTS).map_err(|error| {
            let mut dto = ErrorDto::from(error);
            if dto.kind == "rejected" {
                dto.message = "Бесплатные тестовые SOL сейчас недоступны (у раздачи есть лимит). \
                               Попробуйте позже или пополните адрес вручную."
                    .to_string();
            }
            dto
        })
    })
    .await
}
```

`family-app/src-tauri/src/commands/pairing.rs`:

```rust
//! Handing this key to a kiosk during its first-run setup (Bluetooth).
//! Both commands block for up to about a minute, so they run on a blocking
//! thread. A successful return only means the kiosk accepted the code: it
//! may still fail to register the key on the chain, so the screen must keep
//! checking `list_vaults` until a vault shows up.

use family_app_core::pairing::{pair_with_kiosk, pair_with_kiosk_pin};
use tauri::State;

use super::blocking;
use crate::dto::ErrorDto;
use crate::state::AppState;
use crate::text::pairing_error_dto;

#[tauri::command]
pub async fn pair_with_qr(
    state: State<'_, AppState>,
    qr_text: String,
    name: String,
) -> Result<(), ErrorDto> {
    let me = state.me();
    blocking(move || pair_with_kiosk(&qr_text, &name, me).map_err(|e| pairing_error_dto(&e))).await
}

#[tauri::command]
pub async fn pair_with_pin(
    state: State<'_, AppState>,
    pin: String,
    name: String,
) -> Result<(), ErrorDto> {
    let me = state.me();
    blocking(move || pair_with_kiosk_pin(&pin, &name, me).map_err(|e| pairing_error_dto(&e))).await
}
```

- [ ] **Step 3: Replace `lib.rs` with the final version** (this removes `#![allow(dead_code)]`, so the compiler now proves that nothing is unused)

`family-app/src-tauri/src/lib.rs`:

```rust
//! Trellis Family: the family member's app. A thin Tauri shell around
//! `family-app-core`: `commands` is the IPC surface, `text`/`view` hold
//! every rule and sentence as pure, tested functions, and the `ui/` folder
//! only displays what these return.

mod commands;
mod config;
mod dto;
mod pending;
mod state;
mod text;
mod view;

use std::sync::Arc;

use family_app_core::identity::generate_or_load_identity;
use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use tauri::Manager;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
            let identity = generate_or_load_identity(&data_dir.join(config::IDENTITY_FILE))
                .map_err(|e| e.to_string())?;
            // `confirmed`, so a screen refreshed right after a send shows it.
            let rpc =
                RpcClient::new_with_commitment(config::rpc_url(), CommitmentConfig::confirmed());
            app.manage(AppState {
                identity: Arc::new(identity),
                rpc: Arc::new(rpc),
                program_id: config::program_id(),
                data_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::read::app_info,
            commands::read::wallet,
            commands::read::list_vaults,
            commands::read::load_vault,
            commands::read::load_history,
            commands::read::pending_status,
            commands::read::check_pending,
            commands::actions::deposit,
            commands::actions::send_money,
            commands::actions::add_payee,
            commands::actions::add_guardian,
            commands::actions::approve_request,
            commands::actions::open_family_account,
            commands::actions::get_test_sol,
            commands::pairing::pair_with_qr,
            commands::pairing::pair_with_pin,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Trellis Family app");
}
```

- [ ] **Step 4: Test, lint, build**

Run: `cargo fmt -p family-app && cargo test -p family-app 2>&1 | grep "test result" | head -1 && cargo clippy -p family-app --all-targets -- -D warnings 2>&1 | tail -3 && cargo build -p family-app 2>&1 | tail -1`
Expected: `test result: ok. 31 passed`, no clippy output, and the build finishes. If `dead_code` reports anything, the module or function really is unused: delete it, do not re-add the `allow`.

- [ ] **Step 5: Commit**

```bash
git add family-app
git commit -m "feat(family-app): pending-operation record and the IPC commands"
```

---

### Task 5: UI foundation: safe DOM builder and the pure JavaScript modules

**Files:**
- Create: `family-app/ui/{package.json,dom.js,errors.js,input.js,format.js,router.js}`, `family-app/ui/test/pure.test.js`

**Interfaces:**
- Produces (used by Task 6): `dom.h(tag, props, ...children)` and `dom.clear(el)` (text only, never HTML); `errors.normalizeError(e)` (any rejection becomes `{kind,message,details}`); `input.classifyPairingInput(text)` returns `{kind: "pin"|"qr"|"empty"|"unknown", value?}`; `format.formatTime(unixSeconds, nowMs?, timeZone?)`; `router.{ROUTES, parseRoute(hash)}`.

Dates are the only thing JavaScript formats: money is formatted by Rust (global constraint). `node --test` needs no dependencies (Node 22 or newer; the development machine has 26).

- [ ] **Step 1: Create the test file first and watch it fail**

`family-app/ui/test/pure.test.js`:

```javascript
import test from 'node:test';
import assert from 'node:assert/strict';
import { normalizeError } from '../errors.js';
import { classifyPairingInput } from '../input.js';
import { formatTime } from '../format.js';
import { parseRoute, ROUTES } from '../router.js';

test('an ErrorDto from Rust passes through unchanged', () => {
  const dto = { kind: 'rejected', message: 'Не получилось.', details: 'x' };
  assert.equal(normalizeError(dto), dto);
});

test('anything else becomes an internal error with the original text as details', () => {
  const e = normalizeError(new Error('boom'));
  assert.equal(e.kind, 'internal');
  assert.equal(e.details, 'boom');
  assert.ok(e.message.length > 0);
  assert.equal(normalizeError('plain').details, 'plain');
  assert.equal(normalizeError(undefined).kind, 'internal');
});

test('pairing input: six digits is a code, fvpair1 text is a QR, the rest is not', () => {
  assert.deepEqual(classifyPairingInput(' 123456 '), { kind: 'pin', value: '123456' });
  assert.deepEqual(classifyPairingInput('fvpair1:00112233445566778899aabbccddeeff'), {
    kind: 'qr',
    value: 'fvpair1:00112233445566778899aabbccddeeff',
  });
  assert.equal(classifyPairingInput('').kind, 'empty');
  assert.equal(classifyPairingInput(null).kind, 'empty');
  assert.equal(classifyPairingInput('12345').kind, 'unknown');
  assert.equal(classifyPairingInput('1234567').kind, 'unknown');
  assert.equal(classifyPairingInput('hello').kind, 'unknown');
});

const NOW = Date.UTC(2026, 9, 9, 15, 0, 0); // 9 Oct 2026, 15:00 UTC
const at = (y, mo, d, h, mi) => Date.UTC(y, mo - 1, d, h, mi) / 1000;

test('time: today, earlier this year, other year, unknown', () => {
  assert.equal(formatTime(at(2026, 10, 9, 14, 32), NOW, 'UTC'), 'сегодня, 14:32');
  assert.equal(formatTime(at(2026, 10, 5, 9, 5), NOW, 'UTC'), '5 окт., 09:05');
  assert.equal(formatTime(at(2025, 12, 31, 23, 59), NOW, 'UTC'), '31 дек. 2025, 23:59');
  assert.equal(formatTime(null, NOW, 'UTC'), '');
  assert.equal(formatTime(undefined, NOW, 'UTC'), '');
});

test('time: midnight is 00:00, not 24:00', () => {
  assert.equal(formatTime(at(2026, 10, 9, 0, 0), NOW, 'UTC'), 'сегодня, 00:00');
});

test('routes: known names pass, anything else is home', () => {
  for (const r of ROUTES) assert.equal(parseRoute(`#/${r}`), r);
  assert.equal(parseRoute(''), 'home');
  assert.equal(parseRoute('#/nope'), 'home');
  assert.equal(parseRoute('#/send?x=1'), 'send');
  assert.equal(parseRoute(undefined), 'home');
});
```

Create `family-app/ui/package.json`:

`family-app/ui/package.json`:

```json
{
  "name": "family-app-ui",
  "private": true,
  "type": "module",
  "scripts": {
    "test": "node --test test/*.test.js",
    "smoke": "node tools/smoke.mjs"
  }
}
```

Run: `cd family-app/ui && npm test 2>&1 | tail -6`
Expected: FAIL (`Cannot find module ... errors.js`), because the modules do not exist yet.

- [ ] **Step 2: Create the modules**

`family-app/ui/dom.js`:

```javascript
// Tiny DOM builder. Text is ALWAYS inserted as text nodes, never parsed as
// HTML: names and addresses come from the chain and are untrusted.
export function h(tag, props = {}, ...children) {
  const el = document.createElement(tag);
  for (const [key, value] of Object.entries(props || {})) {
    if (value == null || value === false) continue;
    if (key === 'class') el.className = value;
    else if (key.startsWith('on') && typeof value === 'function') {
      el.addEventListener(key.slice(2), value);
    } else if (value === true) el.setAttribute(key, '');
    else el.setAttribute(key, String(value));
  }
  for (const child of children.flat(Infinity)) {
    if (child == null || child === false) continue;
    el.append(child instanceof Node ? child : document.createTextNode(String(child)));
  }
  return el;
}

export function clear(el) {
  el.replaceChildren();
  return el;
}
```

`family-app/ui/errors.js`:

```javascript
// Every failed command rejects with an ErrorDto from Rust:
// { kind, message, details? }. Anything else is wrapped so screens can rely
// on that shape.
export function normalizeError(e) {
  if (e && typeof e === 'object' && typeof e.kind === 'string' && typeof e.message === 'string') {
    return e;
  }
  const details = e && typeof e === 'object' && 'message' in e ? String(e.message) : String(e);
  return { kind: 'internal', message: 'Что-то пошло не так. Попробуйте ещё раз.', details };
}
```

`family-app/ui/input.js`:

```javascript
// What the person typed or pasted on the first-launch screen: the text of the
// QR code on the device ("fvpair1:..."), or the six-digit backup code.
export function classifyPairingInput(text) {
  const value = String(text ?? '').trim();
  if (value === '') return { kind: 'empty' };
  if (/^\d{6}$/.test(value)) return { kind: 'pin', value };
  if (value.startsWith('fvpair1:')) return { kind: 'qr', value };
  return { kind: 'unknown' };
}
```

`family-app/ui/format.js`:

```javascript
// Dates only. Money is formatted by Rust; JavaScript never does money math.
const MONTHS = ['янв.', 'февр.', 'мар.', 'апр.', 'мая', 'июн.', 'июл.', 'авг.', 'сент.', 'окт.', 'нояб.', 'дек.'];

function parts(ms, timeZone) {
  const f = new Intl.DateTimeFormat('en-GB', {
    timeZone,
    year: 'numeric',
    month: 'numeric',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23',
  });
  const out = {};
  for (const p of f.formatToParts(new Date(ms))) out[p.type] = p.value;
  return out;
}

// "сегодня, 14:32", "5 окт., 09:05", "5 окт. 2025, 09:05"; '' if unknown.
export function formatTime(unixSeconds, nowMs = Date.now(), timeZone = undefined) {
  if (unixSeconds == null) return '';
  const t = parts(unixSeconds * 1000, timeZone);
  const n = parts(nowMs, timeZone);
  const clock = `${t.hour}:${t.minute}`;
  if (t.year === n.year && t.month === n.month && t.day === n.day) return `сегодня, ${clock}`;
  const date = `${Number(t.day)} ${MONTHS[Number(t.month) - 1]}`;
  return t.year === n.year ? `${date}, ${clock}` : `${date} ${t.year}, ${clock}`;
}
```

`family-app/ui/router.js`:

```javascript
// Hash routes: "#/home", "#/send", ... Unknown or empty -> "home".
export const ROUTES = ['home', 'send', 'people', 'approvals', 'me'];

export function parseRoute(hash) {
  const name = String(hash ?? '').replace(/^#\/?/, '').split('?')[0];
  return ROUTES.includes(name) ? name : 'home';
}
```

- [ ] **Step 3: Run the tests**

Run: `cd family-app/ui && npm test 2>&1 | grep -E "ℹ (tests|pass|fail)"`
Expected: `tests 6`, `pass 6`, `fail 0`.

- [ ] **Step 4: Commit**

```bash
git add family-app/ui
git commit -m "feat(family-app): safe DOM builder and pure UI modules with node tests"
```

---

### Task 6: Screens, scripted mock backend, shell and the end-to-end smoke test

**Files:**
- Replace: `family-app/ui/index.html`
- Create: `family-app/ui/{styles.css,api.js,mock-api.js,state.js,components.js,app.js}`, `family-app/ui/screens/{first,home,send,people,approvals,me}.js`, `family-app/ui/tools/smoke.mjs`

**Interfaces:**
- Consumes: the command contract from Task 4 (names and camelCase arguments), and Task 5's `dom`, `errors`, `input`, `format`, `router`.
- Produces: the finished UI. `api.call(command, args)` talks to Tauri, or, when `window.__TAURI__` is missing or the URL has `?mock=...`, to `mock-api.js`, which returns data shaped exactly like `dto.rs`. Mock scenarios: `owner` (default), `guardian`, `empty`, `waiting`, `unconfirmed`, `lowsol`. `components.withFeedback(button, outElement, work)` returns `{ok, value}`; an `unconfirmed` error also fires a `trellis:pending` event so the shell re-reads the pending record.

Behavior this task must get right, each covered by a smoke test below: the screens show only the actions `permissions` allows; a background refresh never rebuilds the screen while the person types (it only re-renders when the data actually changed, otherwise the first-launch form would reset every 5 seconds); names are shown as text, never HTML; a success message survives the re-render that fresh data causes (`ctx.notify`, otherwise "Деньги отправлены" would vanish the moment the balance changes); an unresolved operation shows a persistent warning; a brand-new key can reach its address and the Devnet faucet before any vault exists.

- [ ] **Step 1: Write the smoke test first**

It drives a real headless Chromium over the DevTools protocol (no packages) against the mock backend and serves `ui/` itself. Set `CHROMIUM=/path/to/chromium` if the binary is not called `chromium`.

`family-app/ui/tools/smoke.mjs`:

```javascript
// End-to-end smoke test of the screens against the mock backend, driving a
// real headless Chromium over the DevTools protocol (no extra packages).
//   node tools/smoke.mjs            (serves ui/ itself on a free port)
// Needs a Chromium binary: set CHROMIUM=/path if it is not "chromium".
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { readFile, mkdtemp } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';

const root = fileURLToPath(new URL('..', import.meta.url));
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css' };

const server = createServer(async (req, res) => {
  const path = normalize(new URL(req.url, 'http://x').pathname).replace(/^(\.\.[/\\])+/, '');
  try {
    const body = await readFile(join(root, path === '/' ? 'index.html' : path));
    res.writeHead(200, { 'content-type': TYPES[extname(path)] ?? 'application/octet-stream' }).end(body);
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const base = `http://127.0.0.1:${server.address().port}/index.html`;

const profile = await mkdtemp(join(tmpdir(), 'smoke-'));
const port = 9300 + Math.floor(Math.random() * 500);
const chrome = spawn(process.env.CHROMIUM || 'chromium', [
  '--headless=new', '--no-sandbox', '--disable-gpu', `--remote-debugging-port=${port}`,
  `--user-data-dir=${profile}`, '--window-size=420,900', 'about:blank',
], { stdio: 'ignore' });

async function connect() {
  for (let i = 0; i < 100; i++) {
    try {
      const list = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
      const page = list.find((t) => t.type === 'page');
      if (page) return page.webSocketDebuggerUrl;
    } catch { /* not up yet */ }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error('Chromium did not start');
}

const ws = new WebSocket(await connect());
await new Promise((r) => (ws.onopen = r));
let id = 0;
const pending = new Map();
const errors = [];
ws.onmessage = ({ data }) => {
  const m = JSON.parse(data);
  if (m.id && pending.has(m.id)) pending.get(m.id)(m);
  if (m.method === 'Runtime.exceptionThrown') errors.push(m.params.exceptionDetails.text + ' ' + (m.params.exceptionDetails.exception?.description ?? ''));
  if (m.method === 'Runtime.consoleAPICalled' && m.params.type === 'error') errors.push(m.params.args.map((a) => a.value ?? a.description).join(' '));
};
const send = (method, params = {}) => new Promise((r) => { pending.set(++id, r); ws.send(JSON.stringify({ id, method, params })); });
await send('Runtime.enable');
await send('Page.enable');

const evaluate = async (expression) => {
  const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
  if (r.result.exceptionDetails) throw new Error(r.result.exceptionDetails.exception?.description ?? 'evaluate failed');
  return r.result.result.value;
};
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function until(expression, what, ms = 8000) {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    if (await evaluate(expression)) return;
    await sleep(100);
  }
  throw new Error(`timed out waiting for: ${what}\npage text: ${await evaluate('document.body.innerText')}`);
}
const open = async (scenario, route = 'home') => {
  await send('Page.navigate', { url: `${base}?mock=${scenario}#/${route}` });
  await until("document.getElementById('app').innerText.length > 20", `${scenario} to render`);
};
const text = () => evaluate('document.body.innerText');
const click = (label) => evaluate(`[...document.querySelectorAll('button,a')].find(e => e.textContent.trim() === ${JSON.stringify(label)})?.click()`);
const fill = (selector, value) => evaluate(`(() => { const e = document.querySelector(${JSON.stringify(selector)}); e.focus(); e.value = ${JSON.stringify(value)}; e.dispatchEvent(new Event('input', {bubbles:true})); })()`);

const tests = [];
const test = (name, fn) => tests.push([name, fn]);

test('owner: balance, limit and role are shown', async () => {
  await open('owner');
  const t = await text();
  assert.match(t, /250\.5 USDC/);
  assert.match(t, /ещё 70 USDC из 100 USDC/);
  assert.match(t, /Вы владелец счёта/);
});

test('owner: a small send goes through, a big one becomes a request', async () => {
  await open('owner', 'send');
  await until("document.querySelector('input[name=payee]')", 'payee list');
  await fill("input[type=text]", '25');
  await click('Отправить');
  await until("document.body.innerText.includes('Деньги отправлены.')", 'send ok');
  // The refresh that follows changes the balance and rebuilds the screen: the
  // success message must still be there afterwards.
  await sleep(1200);
  assert.match(await text(), /Деньги отправлены\./);
  await fill("input[type=text]", '80');
  await click('Отправить');
  await until("document.body.innerText.includes('Запрос отправлен')", 'request filed');
});

test('owner: a payee still in the 48h wait is not offered', async () => {
  await open('owner', 'send');
  await until("document.querySelector('input[name=payee]')", 'payee list');
  assert.equal(await evaluate("document.querySelectorAll('input[name=payee]').length"), 1);
  assert.match(await text(), /Сосед Пётр/); // listed as "not yet usable"
});

test('guardian: sees the pending request and approves it', async () => {
  await open('guardian', 'approvals');
  await until("document.body.innerText.includes('Перевод 150 USDC')", 'request card');
  await click('Одобрить');
  await until("document.body.innerText.includes('Перевод одобрен.')", 'approved');
  await until("!document.querySelector('.count')", 'badge gone');
});

test('guardian: cannot send, and is told why', async () => {
  await open('guardian', 'send');
  assert.match(await text(), /только его владелец/);
});

test('first launch: bad text is refused, good text waits for the chain', async () => {
  await open('empty');
  await until("document.body.innerText.includes('Меня добавляет родственник')", 'first screen');
  await click('Передать устройству');
  await until("document.body.innerText.includes('Напишите своё имя')", 'name required');
  await fill('input[autocomplete=name]', 'Внук Саша');
  await fill('input[placeholder^="fvpair1"]', 'hello');
  await click('Передать устройству');
  await until("document.body.innerText.includes('Введите текст QR-кода')", 'bad code refused');
  await fill('input[placeholder^="fvpair1"]', 'fvpair1:00112233445566778899aabbccddeeff');
  await click('Передать устройству');
  await until("document.body.innerText.includes('Ждём, пока вас добавят')", 'waiting');
});

test('first launch: once the vault appears the main screen takes over', async () => {
  await open('waiting');
  await fill('input[autocomplete=name]', 'Внук Саша');
  await fill('input[placeholder^="fvpair1"]', '123456');
  await click('Передать устройству');
  await until("document.body.innerText.includes('Ждём, пока вас добавят')", 'waiting');
  await until("document.body.innerText.includes('Личный счёт')", 'vault shown', 30000);
});

test('first launch: a new key can reach the address and the test-SOL button before any vault exists', async () => {
  await open('empty');
  await until("document.body.innerText.includes('Ваш адрес и комиссии')", 'address card');
  assert.match(await text(), /На адресе: 0 SOL/);
  await click('Получить тестовые SOL');
  await until("document.body.innerText.includes('SOL пришли на ваш адрес')", 'faucet ok');
  await until("document.body.innerText.includes('На адресе: 0.1 SOL')", 'balance updated');
  assert.match(await text(), /SOL пришли на ваш адрес/); // message survived the re-render
});

test('a wrong fallback code shows the plain message and keeps the details folded', async () => {
  await open('empty');
  await fill('input[autocomplete=name]', 'Внук Саша');
  await fill('input[placeholder^="fvpair1"]', '000000');
  await click('Передать устройству');
  await until("document.body.innerText.includes('Устройство не приняло код')", 'rejection message');
  assert.equal(await evaluate("document.querySelector('details')?.open"), false);
});

test('an unconfirmed operation shows the warning until it is checked', async () => {
  await open('unconfirmed');
  await until("document.body.innerText.includes('Не закончена операция')", 'warning');
  await click('Проверить');
  await until("!document.body.innerText.includes('Не закончена операция')", 'warning cleared', 10000);
});

test('a background refresh never wipes what the person is typing', async () => {
  await open('owner', 'send');
  await until("document.querySelector('input[type=text]')", 'amount field');
  await fill('input[type=text]', '12.5');
  await evaluate("document.dispatchEvent(new Event('visibilitychange'))");
  await sleep(1500);
  assert.equal(await evaluate("document.querySelector('input[type=text]').value"), '12.5');
});

test('names from the chain are shown as text, never as HTML', async () => {
  await open('owner', 'people');
  await until("document.body.innerText.includes('Внучка Анна')", 'people list');
  await evaluate(`import('./dom.js').then(({h}) => { document.body.append(h('div', {id:'x'}, '<img src=x onerror=window.__pwned=1>')); })`);
  await sleep(200);
  assert.equal(await evaluate('window.__pwned === undefined && !document.querySelector("#x img")'), true);
});

let failed = 0;
for (const [name, fn] of tests) {
  try {
    await fn();
    console.log(`ok   ${name}`);
  } catch (e) {
    failed++;
    console.log(`FAIL ${name}\n     ${String(e.message).split('\n').join('\n     ')}`);
  }
}
if (errors.length) {
  failed++;
  console.log(`FAIL console errors:\n     ${errors.join('\n     ')}`);
}
chrome.kill();
server.close();
console.log(failed ? `${failed} failed` : 'all passed');
process.exit(failed ? 1 : 0);
```

Run: `cd family-app/ui && CHROMIUM=chromium node tools/smoke.mjs 2>&1 | tail -15`
Expected: failures such as `timed out waiting for: owner to render` (the app does not exist yet).

- [ ] **Step 2: Create the shell, styles and plumbing**

`family-app/ui/index.html`:

```html
<!doctype html>
<html lang="ru">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
    <meta name="color-scheme" content="light dark" />
    <title>Trellis Family</title>
    <link rel="stylesheet" href="styles.css" />
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="app.js"></script>
  </body>
</html>
```

`family-app/ui/styles.css`:

```css
:root {
  --bg: #f6f7f9;
  --surface: #ffffff;
  --text: #15181d;
  --muted: #5a6370;
  --line: #d7dce3;
  --accent: #0b6b4f;
  --accent-text: #ffffff;
  --ok: #0b6b4f;
  --warn-bg: #fff4d6;
  --warn-line: #d9a400;
  --err-bg: #fde8e8;
  --err-line: #c0392b;
  --info-bg: #e8f1fb;
  --info-line: #3b78c2;
  --ok-bg: #e6f5ee;
  --radius: 14px;
  --tap: 52px;
  font-size: 18px;
}

@media (prefers-color-scheme: dark) {
  :root:not([data-theme='light']) {
    --bg: #0f1215;
    --surface: #181c21;
    --text: #eef1f5;
    --muted: #a3acb8;
    --line: #2d343d;
    --accent: #2fbf8f;
    --accent-text: #06231a;
    --ok: #4fd1a3;
    --warn-bg: #3a2f10;
    --warn-line: #d9a400;
    --err-bg: #3a1714;
    --err-line: #e5604f;
    --info-bg: #14263a;
    --info-line: #5a9ae0;
    --ok-bg: #11302a;
  }
}

* { box-sizing: border-box; }

body {
  margin: 0;
  background: var(--bg);
  color: var(--text);
  font-family: system-ui, -apple-system, 'Segoe UI', Roboto, 'Noto Sans', sans-serif;
  line-height: 1.45;
}

#app { min-height: 100vh; display: flex; flex-direction: column; max-width: 560px; margin: 0 auto; }

.top { display: flex; align-items: center; gap: 0.6rem; padding: 0.8rem 1rem; border-bottom: 1px solid var(--line); background: var(--surface); position: sticky; top: 0; }
.top select { margin-left: auto; max-width: 60%; }
.tag { font-size: 0.7rem; padding: 0.1rem 0.5rem; border-radius: 99px; background: var(--warn-bg); border: 1px solid var(--warn-line); }

.content { flex: 1; padding: 1rem 1rem 6rem; display: flex; flex-direction: column; gap: 1rem; }
.stack { display: flex; flex-direction: column; gap: 1rem; }
.center { text-align: center; padding: 3rem 1rem; }
h1 { font-size: 1.7rem; margin: 0.4rem 0; }
h2 { font-size: 1.15rem; margin: 0 0 0.6rem; }
.lead { font-size: 1.05rem; color: var(--muted); margin-top: 0; }
.muted { color: var(--muted); }
.ok { color: var(--ok); font-weight: 600; }
small { font-size: 0.8rem; }

.card { background: var(--surface); border: 1px solid var(--line); border-radius: var(--radius); padding: 1rem; }
.hero { text-align: center; }
.hero p { margin: 0.2rem 0; }
.balance { font-size: 2.4rem; font-weight: 700; letter-spacing: -0.02em; }

.row { display: flex; gap: 0.7rem; flex-wrap: wrap; align-items: center; }
.row .btn { flex: 1 1 8rem; }

.btn { min-height: var(--tap); padding: 0.6rem 1.2rem; border-radius: var(--radius); border: 2px solid var(--accent); background: var(--accent); color: var(--accent-text); font: inherit; font-weight: 600; cursor: pointer; }
.btn-secondary { background: transparent; color: var(--accent); }
.btn:disabled { opacity: 0.5; cursor: progress; }
.btn:focus-visible, a:focus-visible, input:focus-visible, select:focus-visible { outline: 3px solid var(--info-line); outline-offset: 2px; }

.field { margin: 0.8rem 0; }
.field label { display: block; font-weight: 600; margin-bottom: 0.3rem; }
.hint { margin: 0.3rem 0 0; color: var(--muted); font-size: 0.85rem; }
input[type='text'], select { width: 100%; min-height: var(--tap); padding: 0.5rem 0.8rem; font: inherit; color: var(--text); background: var(--bg); border: 2px solid var(--line); border-radius: var(--radius); }
.top select { width: auto; min-height: 40px; }

.choices { border: 0; margin: 0; padding: 0; }
.choice { display: flex; gap: 0.7rem; align-items: center; min-height: var(--tap); padding: 0.3rem 0; }
.choice input { width: 1.4rem; height: 1.4rem; }

.list { list-style: none; margin: 0; padding: 0; }
.list li { display: flex; justify-content: space-between; gap: 0.8rem; padding: 0.7rem 0; border-top: 1px solid var(--line); }
.list li:first-child { border-top: 0; }
.list li > span:last-child { white-space: nowrap; flex-shrink: 0; }

code { font-family: ui-monospace, 'SF Mono', Menlo, monospace; font-size: 0.85rem; word-break: break-all; }
.address { display: block; padding: 0.6rem; background: var(--bg); border-radius: 10px; margin: 0.6rem 0; }

.banner { border: 2px solid; border-radius: var(--radius); padding: 0.7rem 1rem; display: flex; flex-direction: column; gap: 0.6rem; align-items: flex-start; }
.banner p { margin: 0; }
.banner-info { background: var(--info-bg); border-color: var(--info-line); }
.banner-ok { background: var(--ok-bg); border-color: var(--ok); }
.banner-warn { background: var(--warn-bg); border-color: var(--warn-line); }
.banner-error { background: var(--err-bg); border-color: var(--err-line); }
details { font-size: 0.85rem; }

.tabs { position: fixed; bottom: 0; left: 0; right: 0; max-width: 560px; margin: 0 auto; display: flex; background: var(--surface); border-top: 1px solid var(--line); padding-bottom: env(safe-area-inset-bottom); }
.tab { flex: 1; min-height: var(--tap); display: flex; align-items: center; justify-content: center; gap: 0.3rem; color: var(--muted); text-decoration: none; font-size: 0.85rem; padding: 0.3rem; text-align: center; }
.tab.current { color: var(--accent); font-weight: 700; border-top: 3px solid var(--accent); }
.count { background: var(--err-line); color: #fff; border-radius: 99px; padding: 0 0.45rem; font-size: 0.75rem; }

.sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); white-space: nowrap; }

@media (prefers-reduced-motion: no-preference) { .btn { transition: opacity 0.15s; } }
```

`family-app/ui/api.js`:

```javascript
// The only place that talks to the Rust side. Outside Tauri (a plain browser,
// or "?mock=..." in the URL) it falls back to a scripted fake so every screen
// can be looked at without a device or a network.
import { normalizeError } from './errors.js';

const tauri = globalThis.__TAURI__?.core;
const wantsMock = new URLSearchParams(globalThis.location?.search ?? '').has('mock');
export const isMock = !tauri || wantsMock;

let backend;

async function getBackend() {
  if (!backend) {
    backend = isMock ? (await import('./mock-api.js')).default : { invoke: tauri.invoke };
  }
  return backend;
}

export async function call(command, args = {}) {
  const b = await getBackend();
  try {
    return await b.invoke(command, args);
  } catch (e) {
    throw normalizeError(e);
  }
}
```

`family-app/ui/state.js`:

```javascript
// One shared state object; screens read it, `update` notifies the shell.
export const store = {
  info: null, // { address, address_short, network }
  wallet: null, // { sol, low_sol }
  vaults: [], // [{ address, address_short, mine }]
  selected: null, // address of the vault on screen
  vault: null, // VaultDto
  history: [], // [{ signature, text, time }]
  pending: null, // { what, signature } while an operation's outcome is unknown
  notice: null, // text of a success message still on screen
  loaded: false,
  loadError: null, // ErrorDto when the last refresh failed
};

const listeners = new Set();

export function subscribe(fn) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

export function update(patch) {
  Object.assign(store, patch);
  for (const fn of listeners) fn();
}
```

`family-app/ui/components.js`:

```javascript
import { h } from './dom.js';

export function banner(kind, text, ...extra) {
  return h('div', { class: `banner banner-${kind}`, role: kind === 'error' ? 'alert' : 'status' }, h('p', {}, text), ...extra);
}

// An ErrorDto from Rust: the plain message, details tucked away.
export function errorBanner(e) {
  if (!e) return null;
  return banner(
    e.kind === 'unconfirmed' ? 'warn' : 'error',
    e.message,
    e.details ? h('details', {}, h('summary', {}, 'Подробности'), h('code', {}, e.details)) : null,
  );
}

export function button(label, onclick, opts = {}) {
  return h('button', { type: 'button', class: `btn ${opts.secondary ? 'btn-secondary' : ''}`, onclick, disabled: opts.disabled }, label);
}

export function field(label, input, hint) {
  const id = `f-${Math.random().toString(36).slice(2, 8)}`;
  input.id = id;
  return h('div', { class: 'field' }, h('label', { for: id }, label), input, hint ? h('p', { class: 'hint' }, hint) : null);
}

export function card(title, ...children) {
  return h('section', { class: 'card' }, title ? h('h2', {}, title) : null, ...children);
}

// Runs `work` with the button disabled. Returns { ok: true, value } or, after
// showing the error in `out`, { ok: false }. An "unconfirmed" error also tells
// the shell to re-read the pending operation so the warning banner appears.
export async function withFeedback(btn, out, work) {
  btn.disabled = true;
  out.replaceChildren();
  try {
    return { ok: true, value: await work() };
  } catch (e) {
    out.append(errorBanner(e));
    if (e.kind === 'unconfirmed') document.dispatchEvent(new CustomEvent('trellis:pending'));
    return { ok: false, value: null };
  } finally {
    btn.disabled = false;
  }
}
```

`family-app/ui/mock-api.js`:

```javascript
// A scripted stand-in for the Rust commands: same names, same shapes as
// src-tauri/src/dto.rs. Pick a scenario with ?mock=owner|guardian|empty|waiting|unconfirmed|lowsol
const scenario = new URLSearchParams(location.search).get('mock') || 'owner';
const delay = (ms = 250) => new Promise((r) => setTimeout(r, ms));

const ME = '7Hn3mQe1dKq9vXw2YfPz8LbT5aRcUoJs4NgBxVyE6AtM';
const short = (a) => `${a.slice(0, 4)}…${a.slice(-4)}`;

const payees = [
  { name: 'Внучка Анна', address: '9xQeWvG816bUx9EPfHnxZq3JdRkaLo5SyMtBpNwC2Ue', active: true, status: 'Можно отправлять' },
  { name: 'Сосед Пётр', address: 'F3sA7kDmW2cLhVn6TqYbJ9uXoR1ePzG4iKaNvMy8CdBt', active: false, status: 'Можно будет через 31 час' },
].map((p) => ({ ...p, address_short: short(p.address) }));

const guardians = [
  { address: ME, is_me: true },
  { address: 'B6nVr2KxQm8LdYt4WcEhUs1JzPa9GfTo7iXbNe3MvRcA', is_me: false },
].map((g) => ({ ...g, address_short: short(g.address) }));

let pool = {
  address: '4kTzLp9QwXy2VbN7mRcEa1JdUhGs6oFiMt3BnYvKe8Ax',
  title: 'Семейный счёт',
  is_pool: true,
  is_owner: true,
  is_guardian: false,
  balance: '250.5 USDC',
  decimals: 6,
  daily_limit: '100 USDC',
  remaining_today: '70 USDC',
  payees,
  guardians,
  pending: [],
  permissions: { deposit: true, send: true, add_payee: true, add_guardian: true, approve: false },
};
pool.address_short = short(pool.address);

const grandma = {
  address: 'Cw5RnE8yMq2KdLtV7XbZa3JhUp9GfoSi1TmBvNeY4AkD',
  title: 'Личный счёт',
  is_pool: false,
  is_owner: false,
  is_guardian: true,
  balance: '1200 USDC',
  decimals: 6,
  daily_limit: null,
  remaining_today: null,
  payees: payees.slice(0, 1),
  guardians: guardians.slice(0, 1),
  pending: [{ nonce: 3, amount: '150 USDC', to: payees[0].address, to_short: payees[0].address_short, requested_at: Math.floor(Date.now() / 1000) - 600 }],
  permissions: { deposit: true, send: false, add_payee: false, add_guardian: false, approve: true },
};
grandma.address_short = short(grandma.address);

const now = () => Math.floor(Date.now() / 1000);
const history = () => [
  { signature: 's1', text: 'Пополнение на 50 USDC', time: now() - 3600 },
  { signature: 's2', text: 'Отправлено 12.5 USDC', time: now() - 86400 * 2 },
  { signature: 's3', text: 'Добавлен получатель «Внучка Анна»', time: now() - 86400 * 9 },
  { signature: 's4', text: 'Счёт открыт', time: now() - 86400 * 40 },
];

let vaults = { owner: [pool], guardian: [grandma], empty: [], waiting: [], unconfirmed: [pool], lowsol: [pool] }[scenario] ?? [pool];
let solGot = false;
let pendingTx = scenario === 'unconfirmed' ? { what: 'Отправка 5 USDC', signature: 'sig123' } : null;
let waitingPolls = 0;
let moved = 0; // every deposit/send changes the balance, as on a real chain
const setBalance = () => {
  pool.balance = `${(250.5 + moved).toFixed(2).replace(/\.?0+$/, '')} USDC`;
};

const err = (kind, message, details) => ({ kind, message, details: details ?? null });

const handlers = {
  app_info: () => ({ address: ME, address_short: short(ME), network: 'devnet' }),
  wallet: () => (!solGot && ['lowsol', 'empty', 'waiting'].includes(scenario) ? { sol: '0', low_sol: true } : { sol: '0.1', low_sol: false }),
  list_vaults: () => {
    if (scenario === 'waiting' && ++waitingPolls >= 3) vaults = [grandma];
    return vaults.map((v) => ({ address: v.address, address_short: v.address_short, mine: v.is_owner }));
  },
  load_vault: ({ vault }) => {
    const v = vaults.find((x) => x.address === vault);
    if (!v) throw err('rejected', 'Не получилось. Ничего не списано.');
    return v;
  },
  load_history: () => history(),
  pending_status: () => pendingTx,
  check_pending: () => {
    if (!pendingTx) return { status: 'none', message: 'Ничего не ожидает проверки.' };
    pendingTx = null;
    return { status: 'confirmed', message: 'Операция прошла.' };
  },
  deposit: () => {
    moved += 10;
    setBalance();
    return { signature: 'sigD', message: 'Деньги отправлены на счёт.' };
  },
  send_money: ({ amount }) => {
    if (Number(amount) > 70) {
      return { signature: 'sigR', message: 'Это больше суммы, доступной за день. Запрос отправлен: деньги уйдут, когда доверенное лицо его одобрит.' };
    }
    moved -= Number(amount);
    setBalance();
    return { signature: 'sigS', message: 'Деньги отправлены.' };
  },
  add_payee: () => ({ signature: 'sigP', message: 'Получатель добавлен. Пользоваться им можно будет через 48 часов.' }),
  add_guardian: () => ({ signature: 'sigG', message: 'Доверенное лицо добавлено.' }),
  approve_request: () => {
    grandma.pending = [];
    grandma.permissions.approve = false;
    return { signature: 'sigA', message: 'Перевод одобрен.' };
  },
  open_family_account: () => {
    vaults = [pool];
    return { address: pool.address, address_short: pool.address_short, mine: true };
  },
  get_test_sol: () => {
    solGot = true;
    return null;
  },
  // Tauri 2 passes JavaScript arguments in camelCase (Rust: qr_text -> qrText).
  pair_with_qr: ({ qrText }) => {
    if (!String(qrText).startsWith('fvpair1:')) throw err('invalid', 'Это не похоже на код с экрана устройства. Наведите камеру на QR-код ещё раз.');
    return null;
  },
  pair_with_pin: ({ pin }) => {
    if (pin === '000000') throw err('rejected', 'Устройство не приняло код. Проверьте код: на экране мог появиться новый.', 'att 0x0e');
    return null;
  },
};

export default {
  async invoke(command, args) {
    await delay();
    const handler = handlers[command];
    if (!handler) throw err('internal', `Неизвестная команда ${command}`);
    return handler(args ?? {});
  },
};
```

`family-app/ui/app.js`:

```javascript
// The shell: loads state, shows a screen, keeps it fresh.
import { call, isMock } from './api.js';
import { banner, button, errorBanner } from './components.js';
import { clear, h } from './dom.js';
import { parseRoute } from './router.js';
import { store, subscribe, update } from './state.js';
import * as approvals from './screens/approvals.js';
import * as first from './screens/first.js';
import * as home from './screens/home.js';
import * as me from './screens/me.js';
import * as people from './screens/people.js';
import * as send from './screens/send.js';

const SCREENS = { home, send, people, approvals, me };
const TABS = [
  ['home', 'Счёт'],
  ['send', 'Отправить'],
  ['people', 'Люди'],
  ['approvals', 'Одобрения'],
  ['me', 'Я'],
];
const REFRESH_MS = 60_000;
const NOTICE_MS = 10_000;
const SELECTED_KEY = 'trellis.selectedVault';

const root = document.getElementById('app');
let leaveHooks = [];
let noticeTimer;

const ctx = {
  store,
  refresh: (options) => loadAll(options),
  go: (route) => {
    location.hash = `#/${route}`;
  },
  onLeave: (fn) => leaveHooks.push(fn),
  // A success message that outlives the re-render that fresh data causes.
  notify: (text) => {
    store.notice = text;
    clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => {
      store.notice = null;
      document.getElementById('notice')?.remove(); // no re-render: nobody's typing is interrupted
    }, NOTICE_MS);
    document.getElementById('notice')?.replaceWith(noticeBanner() ?? document.createComment(''));
    if (!document.getElementById('notice')) render();
  },
};

function remember(address) {
  try {
    localStorage.setItem(SELECTED_KEY, address);
  } catch {
    /* storage may be blocked; the first vault is a fine default */
  }
}

function remembered() {
  try {
    return localStorage.getItem(SELECTED_KEY);
  } catch {
    return null;
  }
}

async function loadPending() {
  try {
    update({ pending: await call('pending_status') });
  } catch (e) {
    update({ loadError: e });
  }
}

// True while the person is typing in a field: a background refresh must not
// rebuild the screen under their fingers.
function isTyping() {
  const el = document.activeElement;
  return Boolean(el && root.contains(el) && ['INPUT', 'SELECT', 'TEXTAREA'].includes(el.tagName));
}

let lastSnapshot = '';

// `background` refreshes (timers, returning to the app) change nothing on
// screen unless the data really changed, and never interrupt typing.
async function loadAll({ background = false } = {}) {
  try {
    const [wallet, vaults] = await Promise.all([call('wallet'), call('list_vaults')]);
    let selected = store.selected;
    if (!vaults.some((v) => v.address === selected)) {
      selected = vaults.find((v) => v.address === remembered())?.address ?? vaults[0]?.address ?? null;
    }
    let vault = null;
    let history = [];
    if (selected) {
      vault = await call('load_vault', { vault: selected });
      history = await call('load_history', { vault: selected, decimals: vault.decimals });
      remember(selected);
    }
    const snapshot = JSON.stringify({ wallet, vaults, selected, vault, history });
    const unchanged = store.loaded && !store.loadError && snapshot === lastSnapshot;
    if (unchanged || (background && isTyping())) return;
    lastSnapshot = snapshot;
    update({ wallet, vaults, selected, vault, history, loaded: true, loadError: null });
  } catch (e) {
    if (background && isTyping()) return;
    update({ loaded: true, loadError: e });
  }
}

function header() {
  const picker =
    store.vaults.length > 1
      ? h(
          'select',
          {
            'aria-label': 'Выбрать счёт',
            onchange: (ev) => {
              update({ selected: ev.target.value });
              loadAll();
            },
          },
          store.vaults.map((v) => h('option', { value: v.address, selected: v.address === store.selected }, `${v.mine ? 'Мой счёт' : 'Счёт родственника'} ${v.address_short}`)),
        )
      : null;
  return h('header', { class: 'top' }, h('strong', {}, 'Trellis'), isMock ? h('span', { class: 'tag' }, 'демо') : null, picker);
}

function pendingBanner() {
  if (!store.pending) return null;
  const out = h('div', { 'aria-live': 'polite' });
  const check = button('Проверить', async () => {
    check.disabled = true;
    try {
      const r = await call('check_pending');
      out.replaceChildren(banner(r.status === 'waiting' ? 'warn' : 'info', r.message));
      await loadPending();
      if (r.status !== 'waiting') await loadAll();
    } catch (e) {
      out.replaceChildren(errorBanner(e));
    } finally {
      check.disabled = false;
    }
  });
  return banner('warn', `Не закончена операция: «${store.pending.what}». Пока мы не узнаем, чем она кончилась, другие переводы недоступны, чтобы не заплатить дважды.`, check, out);
}

function noticeBanner() {
  if (!store.notice) return null;
  const el = banner('ok', store.notice);
  el.id = 'notice';
  return el;
}

function nav(current) {
  const waiting = store.vault?.permissions.approve ? store.vault.pending.length : 0;
  return h(
    'nav',
    { class: 'tabs', 'aria-label': 'Разделы' },
    TABS.map(([name, label]) =>
      h('a', { href: `#/${name}`, class: name === current ? 'tab current' : 'tab', 'aria-current': name === current ? 'page' : null }, label, name === 'approvals' && waiting ? h('span', { class: 'count' }, waiting) : null),
    ),
  );
}

function render() {
  for (const fn of leaveHooks) fn();
  leaveHooks = [];
  clear(root);
  if (!store.loaded) return root.append(h('p', { class: 'center' }, 'Загрузка…'));

  if (!store.vault) {
    root.append(h('main', { class: 'content' }, noticeBanner(), store.loadError ? errorBanner(store.loadError) : null, pendingBanner(), first.render(ctx)));
    return;
  }
  const route = parseRoute(location.hash);
  root.append(
    header(),
    h('main', { class: 'content' }, noticeBanner(), store.loadError ? errorBanner(store.loadError) : null, pendingBanner(), SCREENS[route].render(ctx)),
    nav(route),
  );
}

subscribe(render);
window.addEventListener('hashchange', render);
document.addEventListener('trellis:pending', loadPending);
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'visible') loadAll({ background: true });
});
setInterval(() => {
  if (document.visibilityState === 'visible') loadAll({ background: true });
}, REFRESH_MS);

(async () => {
  render();
  try {
    update({ info: await call('app_info') });
  } catch (e) {
    update({ loadError: e });
  }
  await loadPending();
  await loadAll();
})();
```

- [ ] **Step 3: Create the screens**

`family-app/ui/screens/first.js`:

```javascript
// First launch: this phone has no vault yet. Two ways in:
//  1. be added by a kiosk during ITS first-run setup (type what the kiosk
//     shows), then wait until the chain says so;
//  2. open a family account of your own.
import { call } from '../api.js';
import { button, card, errorBanner, field, banner, withFeedback } from '../components.js';
import { h } from '../dom.js';
import { classifyPairingInput } from '../input.js';

export function render(ctx) {
  const { store } = ctx;
  const root = h('div', { class: 'stack' });
  let waiting = false;
  let timer = null;

  const stopWaiting = () => clearInterval(timer);

  function draw() {
    root.replaceChildren(
      h('h1', {}, 'Добро пожаловать'),
      h('p', { class: 'lead' }, 'Это приложение для близких: оно помогает следить за счётом родственника и одобрять переводы.'),
      waiting ? waitingCard() : pairCard(),
      openCard(),
      addressCard(),
    );
  }

  function pairCard() {
    const name = h('input', { type: 'text', maxlength: 24, autocomplete: 'name', placeholder: 'Например: Внук Саша' });
    const code = h('input', { type: 'text', inputmode: 'text', autocomplete: 'off', placeholder: 'fvpair1:… или 6 цифр' });
    const out = h('div', { 'aria-live': 'polite' });
    const go = button('Передать устройству', async () => {
      const input = classifyPairingInput(code.value);
      if (!name.value.trim()) return out.replaceChildren(errorBanner({ kind: 'invalid', message: 'Напишите своё имя: так вас увидит родственник на экране.' }));
      if (input.kind === 'empty' || input.kind === 'unknown') {
        return out.replaceChildren(errorBanner({ kind: 'invalid', message: 'Введите текст QR-кода с экрана устройства или 6 цифр под ним.' }));
      }
      const r = await withFeedback(go, out, () =>
        input.kind === 'pin'
          ? call('pair_with_pin', { pin: input.value, name: name.value.trim() })
          : call('pair_with_qr', { qrText: input.value, name: name.value.trim() }),
      );
      if (!r.ok) return;
      // The kiosk accepted the code, but it may still fail to register us on
      // the chain: keep looking until a vault actually shows up.
      waiting = true;
      draw();
      timer = setInterval(() => ctx.refresh({ background: true }), 5000);
    });
    return card(
      'Меня добавляет родственник',
      h('p', {}, 'Попросите включить настройку на устройстве родственника. На его экране появится QR-код и 6 цифр. Станьте рядом, включите Bluetooth.'),
      field('Ваше имя', name, 'Родственник увидит его на экране устройства.'),
      field('Код с экрана', code, 'Текст QR-кода или 6 цифр под ним.'),
      go,
      out,
    );
  }

  function waitingCard() {
    return card(
      'Ждём, пока вас добавят',
      banner('info', 'Устройство приняло ваш код. Оно записывает вас в список, это занимает около минуты. Экран обновится сам.'),
      button('Проверить сейчас', () => ctx.refresh(), { secondary: true }),
    );
  }

  function openCard() {
    const limit = h('input', { type: 'text', inputmode: 'decimal', placeholder: 'Например: 100' });
    const out = h('div', { 'aria-live': 'polite' });
    const go = button('Открыть семейный счёт', async () => {
      const r = await withFeedback(go, out, () => call('open_family_account', { dailyLimit: limit.value }));
      if (r.ok) await ctx.refresh();
    });
    return card(
      'Или открыть свой семейный счёт',
      h('p', {}, 'Общий счёт семьи в долларах (USDC). Чтобы тратить с него, нужны получатели и доверенные лица.'),
      field('Сколько можно отправить за день без одобрения (USDC)', limit),
      go,
      out,
    );
  }

  // A brand-new key has no SOL, and every action costs a small fee, so the
  // address and the Devnet faucet are reachable before any vault exists.
  function addressCard() {
    const out = h('div', { 'aria-live': 'polite' });
    const faucet = button(
      'Получить тестовые SOL',
      async () => {
        const r = await withFeedback(faucet, out, () => call('get_test_sol'));
        if (r.ok) {
          ctx.notify('Готово: SOL пришли на ваш адрес.');
          await ctx.refresh();
        }
      },
      { secondary: true },
    );
    return card(
      'Ваш адрес и комиссии',
      h('p', {}, 'Каждое действие стоит совсем немного SOL. Родственнику этот адрес нужен, чтобы вас добавить.'),
      h('code', { class: 'address' }, store.info?.address ?? ''),
      h('p', {}, `На адресе: ${store.wallet?.sol ?? '…'} SOL`),
      store.wallet?.low_sol ? banner('warn', 'SOL почти нет. Получите тестовые SOL, иначе счёт не открыть.') : null,
      faucet,
      out,
    );
  }

  draw();
  ctx.onLeave(stopWaiting);
  return root;
}
```

`family-app/ui/screens/home.js`:

```javascript
import { call } from '../api.js';
import { banner, button, card, errorBanner, field, withFeedback } from '../components.js';
import { h } from '../dom.js';
import { formatTime } from '../format.js';

export function render(ctx) {
  const { store } = ctx;
  const v = store.vault;
  const root = h('div', { class: 'stack' });

  const role = v.is_owner ? 'Вы владелец счёта' : v.is_guardian ? 'Вы доверенное лицо' : 'Вы можете пополнять счёт';
  root.append(
    h('section', { class: 'card hero' },
      h('p', { class: 'muted' }, v.title),
      h('p', { class: 'balance', 'aria-label': `Баланс: ${v.balance}` }, v.balance),
      v.remaining_today ? h('p', {}, `Сегодня можно отправить ещё ${v.remaining_today}`, v.daily_limit ? ` из ${v.daily_limit}` : '') : null,
      h('p', { class: 'muted' }, role),
    ),
  );

  if (v.permissions.approve) {
    root.append(banner('warn', `Ждут вашего одобрения: ${v.pending.length}`, button('Посмотреть', () => ctx.go('approvals'))));
  } else if (v.pending.length > 0) {
    root.append(banner('info', `Запросов на перевод, которые ждут одобрения доверенного лица: ${v.pending.length}`));
  }

  const actions = h('div', { class: 'row' });
  if (v.permissions.send) actions.append(button('Отправить', () => ctx.go('send')));
  actions.append(button('Пополнить', () => toggleDeposit(), { secondary: v.permissions.send }));
  root.append(actions);

  const depositBox = h('div');
  root.append(depositBox);

  function toggleDeposit() {
    if (depositBox.childElementCount) return depositBox.replaceChildren();
    const amount = h('input', { type: 'text', inputmode: 'decimal', placeholder: 'Например: 25 или 2.50' });
    const out = h('div', { 'aria-live': 'polite' });
    const go = button('Пополнить', async () => {
      const r = await withFeedback(go, out, () => call('deposit', { vault: v.address, amount: amount.value }));
      if (r.ok) {
        amount.value = '';
        ctx.notify(r.value.message);
        await ctx.refresh();
      }
    });
    depositBox.append(card('Пополнить счёт', field('Сумма (USDC)', amount, 'Деньги берутся с вашего адреса. Тестовые USDC можно получить на faucet.circle.com.'), go, out));
  }

  const list = h('ul', { class: 'list' });
  if (store.history.length === 0) list.append(h('li', { class: 'muted' }, 'Пока ничего не происходило.'));
  for (const item of store.history.slice(0, 10)) {
    list.append(h('li', {}, h('span', {}, item.text), h('span', { class: 'muted' }, formatTime(item.time))));
  }
  root.append(card('Последние операции', list));
  return root;
}
```

`family-app/ui/screens/send.js`:

```javascript
import { call } from '../api.js';
import { banner, button, card, field, withFeedback } from '../components.js';
import { h } from '../dom.js';

export function render(ctx) {
  const v = ctx.store.vault;
  if (!v.permissions.send) {
    const why = !v.is_owner
      ? 'Отправлять деньги с этого счёта может только его владелец.'
      : 'Пока некому отправлять: нового получателя можно использовать через 48 часов после добавления.';
    return card('Отправить', banner('info', why), button('Назад', () => ctx.go('home'), { secondary: true }));
  }

  const ready = v.payees.filter((p) => p.active);
  const chosen = { address: ready[0]?.address };
  const amount = h('input', { type: 'text', inputmode: 'decimal', placeholder: 'Например: 25 или 2.50' });
  const out = h('div', { 'aria-live': 'polite' });

  const options = ready.map((p, i) =>
    h('label', { class: 'choice' },
      h('input', { type: 'radio', name: 'payee', value: p.address, checked: i === 0, onchange: () => (chosen.address = p.address) }),
      h('span', {}, p.name, h('small', { class: 'muted' }, ` ${p.address_short}`)),
    ),
  );

  const go = button('Отправить', async () => {
    const r = await withFeedback(go, out, () => call('send_money', { vault: v.address, recipient: chosen.address, amount: amount.value }));
    if (r.ok) {
      amount.value = '';
      ctx.notify(r.value.message);
      await ctx.refresh();
    }
  });

  const notReady = v.payees.filter((p) => !p.active);
  return h('div', { class: 'stack' },
    card('Кому отправить', h('fieldset', { class: 'choices' }, h('legend', { class: 'sr-only' }, 'Получатель'), options)),
    card('Сколько', field('Сумма (USDC)', amount, v.remaining_today ? `Сегодня без одобрения можно ещё ${v.remaining_today}.` : null), go, out),
    notReady.length ? banner('info', `Ещё нельзя использовать: ${notReady.map((p) => `${p.name} (${p.status.toLowerCase()})`).join(', ')}.`) : null,
  );
}
```

`family-app/ui/screens/people.js`:

```javascript
import { call } from '../api.js';
import { banner, button, card, field, withFeedback } from '../components.js';
import { h } from '../dom.js';

export function render(ctx) {
  const v = ctx.store.vault;
  const root = h('div', { class: 'stack' });

  const payees = h('ul', { class: 'list' });
  if (v.payees.length === 0) payees.append(h('li', { class: 'muted' }, 'Получателей пока нет.'));
  for (const p of v.payees) {
    payees.append(h('li', {}, h('span', {}, h('strong', {}, p.name), h('small', { class: 'muted' }, ` ${p.address_short}`)), h('span', { class: p.active ? 'ok' : 'muted' }, p.status)));
  }
  root.append(card('Получатели', payees));

  if (v.permissions.add_payee) {
    const name = h('input', { type: 'text', maxlength: 16, placeholder: 'Например: Внучка Анна' });
    const address = h('input', { type: 'text', autocomplete: 'off', spellcheck: 'false', placeholder: 'Адрес получателя' });
    const out = h('div', { 'aria-live': 'polite' });
    const go = button('Добавить получателя', async () => {
      const r = await withFeedback(go, out, () => call('add_payee', { vault: v.address, name: name.value, address: address.value }));
      if (r.ok) {
        name.value = '';
        address.value = '';
        ctx.notify(r.value.message);
        await ctx.refresh();
      }
    });
    root.append(card('Добавить получателя', field('Имя', name), field('Адрес', address, 'Человек присылает вам свой адрес со вкладки «Я».'), go, out));
  }

  const guardians = h('ul', { class: 'list' });
  if (v.guardians.length === 0) guardians.append(h('li', { class: 'muted' }, 'Доверенных лиц пока нет.'));
  for (const g of v.guardians) {
    guardians.append(h('li', {}, h('code', {}, g.address_short), g.is_me ? h('span', { class: 'ok' }, 'это вы') : null));
  }
  root.append(card('Доверенные лица', h('p', { class: 'muted' }, 'Они одобряют переводы, которые больше дневного лимита.'), guardians));

  if (v.permissions.add_guardian) {
    const address = h('input', { type: 'text', autocomplete: 'off', spellcheck: 'false', placeholder: 'Адрес доверенного лица' });
    const out = h('div', { 'aria-live': 'polite' });
    const go = button('Добавить доверенное лицо', async () => {
      const r = await withFeedback(go, out, () => call('add_guardian', { vault: v.address, address: address.value }));
      if (r.ok) {
        address.value = '';
        ctx.notify(r.value.message);
        await ctx.refresh();
      }
    });
    root.append(card('Добавить доверенное лицо', field('Адрес', address), go, out));
  } else if (v.is_owner) {
    root.append(banner('info', 'Добавить ещё одно доверенное лицо с этого устройства нельзя: нужна подпись уже добавленного.'));
  }
  return root;
}
```

`family-app/ui/screens/approvals.js`:

```javascript
import { call } from '../api.js';
import { banner, button, card, withFeedback } from '../components.js';
import { h } from '../dom.js';
import { formatTime } from '../format.js';

export function render(ctx) {
  const v = ctx.store.vault;
  if (v.pending.length === 0) return card('Одобрения', banner('info', 'Сейчас нет переводов, которые ждут одобрения.'));

  const items = v.pending.map((r) => {
    const out = h('div', { 'aria-live': 'polite' });
    const approve = button('Одобрить', async () => {
      const result = await withFeedback(approve, out, () => call('approve_request', { vault: v.address, nonce: r.nonce }));
      if (result.ok) {
        ctx.notify(result.value.message);
        await ctx.refresh();
      }
    }, { disabled: !v.permissions.approve });
    return card(
      `Перевод ${r.amount}`,
      h('p', {}, 'Получатель: ', h('code', {}, r.to_short)),
      h('p', { class: 'muted' }, `Запрошен: ${formatTime(r.requested_at)}`),
      v.permissions.approve ? approve : banner('info', 'Одобрить может только доверенное лицо.'),
      out,
    );
  });
  return h('div', { class: 'stack' }, ...items);
}
```

`family-app/ui/screens/me.js`:

```javascript
import { call } from '../api.js';
import { banner, button, card, withFeedback } from '../components.js';
import { h } from '../dom.js';

export function render(ctx) {
  const { store } = ctx;
  const out = h('div', { 'aria-live': 'polite' });
  const copyOut = h('span', { class: 'muted' });

  const copy = button('Скопировать адрес', async () => {
    try {
      await navigator.clipboard.writeText(store.info.address);
      copyOut.textContent = ' Скопировано';
    } catch {
      copyOut.textContent = ' Не удалось скопировать: выделите адрес и скопируйте вручную.';
    }
  }, { secondary: true });

  const faucet = button('Получить тестовые SOL', async () => {
    const r = await withFeedback(faucet, out, () => call('get_test_sol'));
    if (r.ok) {
      ctx.notify('Готово: SOL пришли на ваш адрес.');
      await ctx.refresh();
    }
  });

  return h('div', { class: 'stack' },
    card('Ваш адрес', h('p', {}, 'Так вас находят в сети. Этот адрес можно отправлять родственникам, чтобы они добавили вас получателем.'), h('code', { class: 'address' }, store.info.address), h('div', { class: 'row' }, copy, copyOut)),
    card('Комиссии', h('p', {}, `На адресе: ${store.wallet?.sol ?? '…'} SOL`), store.wallet?.low_sol ? banner('warn', 'SOL почти нет. Без них нельзя ничего отправить или одобрить.') : null, faucet, out),
    card('О приложении', h('p', { class: 'muted' }, `Сеть: ${store.info.network}. Это учебный прототип без аудита: используйте только тестовые деньги.`)),
  );
}
```

- [ ] **Step 4: Syntax check, then the smoke test**

Run: `cd family-app/ui && for f in *.js screens/*.js tools/*.mjs; do node --check $f || echo BAD $f; done; CHROMIUM=chromium node tools/smoke.mjs 2>&1 | tail -14`
Expected: no `BAD`, then 12 lines starting `ok`, then `all passed`.

- [ ] **Step 5: Look at it**

Serve the folder and open each scenario at phone width (about 420 px), in light and dark mode:

```bash
cd family-app/ui && python3 -m http.server 8765
# then open, one by one:
#   http://localhost:8765/index.html?mock=owner#/home        balance, daily allowance, Send + Deposit
#   http://localhost:8765/index.html?mock=guardian#/home     amber "waiting for your approval" banner, no Send
#   http://localhost:8765/index.html?mock=empty              first-launch screen with two cards
#   http://localhost:8765/index.html?mock=unconfirmed#/home  amber "operation not finished" banner on top
```

Check: text is large and readable, every button is at least 52 px tall, nothing overflows horizontally, the focus ring is visible with Tab, and no screen contains the words "vault", "mint", "approver", "beneficiary" or "nonce".

- [ ] **Step 6: Commit**

```bash
git add family-app/ui
git commit -m "feat(family-app): screens, mock backend and an end-to-end smoke test"
```

---

### Task 7: README, whole-workspace verification, and the manual checks that need real hardware

**Files:**
- Create: `family-app/README.md`

This task changes no behavior. It documents the app, proves nothing else in the workspace broke, launches the real app once, and lists the checks that cannot be automated.

- [ ] **Step 1: Create `family-app/README.md`**

````markdown
# Trellis Family

The family member's app for the Trellis family vault: see the balance and recent
operations, deposit, send to approved payees, add payees and guardians, approve
over-limit transfers, and hand your key to a kiosk during its first-run setup.
A Tauri 2 shell around `tools/family-app-core`, with a dependency-free UI.

Devnet prototype, not audited. Use test money only.

## Run

Linux needs the WebKitGTK development packages (`webkit2gtk-4.1`,
`javascriptcoregtk-4.1`, `libsoup-3.0`, `gtk+-3.0`, `librsvg-2.0`, `openssl`).

```
cargo run -p family-app
```

On first launch the app creates its own key in the OS app-data directory
(`~/.local/share/dev.trellis.family/identity.json` on Linux). There is no seed
phrase and no import. The key file is not encrypted at rest in this prototype.
`TRELLIS_RPC_URL` overrides the Devnet endpoint.

A new key has no SOL, and every action costs a tiny fee: use "Получить тестовые
SOL" on the first screen (the public faucet is rate-limited; you can also send
SOL to the shown address by hand). Test USDC comes from faucet.circle.com.

## See the screens without a device or a network

The UI falls back to a scripted backend outside Tauri:

```
cd family-app/ui && python3 -m http.server 8765
# http://localhost:8765/index.html?mock=owner      (also: guardian, empty, waiting, unconfirmed, lowsol)
```

## Tests

```
cargo test -p family-app                 # permissions, sentences, pending-operation record
cd family-app/ui && npm test             # pure JavaScript modules
cd family-app/ui && CHROMIUM=chromium npm run smoke   # every screen, headless Chromium, mock backend
```

## Where the rules live

- `src-tauri/src/view.rs`: who may do what (`permissions`), and whether a send is
  direct, a request above the daily limit, or refused.
- `src-tauri/src/text.rs`: every sentence the person reads, in Russian.
- `src-tauri/src/pending.rs` and `commands/mod.rs`: while an operation's outcome
  is unknown, nothing else may move money.
- The UI never does money math: amounts are typed as text and parsed in Rust.

## Not done yet

Android and Windows builds, camera QR scanning, urgent add (needs two signers),
owner rotation, notifications. See `docs/superpowers/plans/2026-10-10-family-app-tauri-ui.md`.
````

- [ ] **Step 2: Verify the whole workspace**

Run:

```bash
cargo fmt --check -p family-app -p family-app-core -p family-vault-client
cargo clippy -p family-app -p family-app-core -p family-vault-client --all-targets -- -D warnings
cargo test -p family-app -p family-app-core -p family-vault-client -p kiosk-gui -p admin-cli
(cd family-app/ui && npm test && CHROMIUM=chromium npm run smoke)
```

Expected: no fmt or clippy output; every `test result` line `ok` (family-app 31 passed, family-app-core 86 passed, family-vault-client 52 passed, kiosk-gui 71 passed); `npm test` 6 passed; smoke `all passed`. The admin-cli, kiosk-gui and program crates must be unchanged: `git status` shows nothing outside `family-app/`, `tools/family-app-core/`, `Cargo.toml`, `Cargo.lock`, `.gitignore` and the one spec file.

- [ ] **Step 3: Launch the real app once**

On a machine with a display run `cargo run -p family-app`. Expected within about 15 seconds on Devnet: the first-launch screen ("Добро пожаловать") with two cards, the key's real address under "Ваш адрес и комиссии", `На адресе: 0 SOL`, and NO "демо" tag in the header (the tag means the mock backend is active, i.e. IPC is not connected). If the screen stays on "Загрузка…", IPC or the node is the problem: check `TRELLIS_RPC_URL` and the terminal output. This was exactly the failure that exposed the 22-second discovery before Task 1.

- [ ] **Step 4: Manual checks that need hardware or a funded key (record the result of each in the commit message or a follow-up note)**

  - **Kiosk pairing (needs a kiosk in first-run setup, Bluetooth on):** type the QR text from the kiosk screen and a name; expect "Ждём, пока вас добавят", then within about a minute the main screen. Repeat with the 6-digit code. A wrong code must show "Устройство не приняло код" (the kiosk answers with an ATT error because the write is with response). Five wrong codes make the kiosk rotate credentials and start a cooldown: a later correct old code must still fail.
  - **Fees:** on a fresh key press "Получить тестовые SOL"; expect the balance to rise. If the faucet is rate-limited, the message must say so and offer the manual route.
  - **Open a family account (funded key):** enter a daily limit such as `100`; expect the main screen to appear with title "Семейный счёт" and the owner role.
  - **Deposit:** with test USDC in the wallet, deposit `2.50`; expect the balance to change and the green message to stay visible after the refresh.
  - **Add a payee, then send:** add a payee (name + a second key's address); the payee must show "Можно будет через 48 часов" and must NOT be offered on the Send screen. (Waiting 48 hours is impractical: cover the send itself with a vault whose payee is already active, for example one created by `admin-cli` earlier.)
  - **Over-limit request and approval:** from the owner, send more than the remaining daily amount; expect "Запрос отправлен". Open the app as a guardian (a second key added with "Добавить доверенное лицо"); expect the amber banner and the "Одобрения" tab badge; approve; expect the funds to arrive.
  - **Unconfirmed path:** start a send, then cut the network right after pressing the button. Expect the amber "Не закончена операция" banner, all money buttons to refuse with "Сначала нужно узнать...", and after restarting the app the banner must still be there. "Проверить" with the network back must end in "Операция прошла" or "не выполнилась", after which sending works again.
  - **Hostile name:** add a payee whose name contains a newline or a right-to-left override; it must appear sanitised, never reordering the screen text.

- [ ] **Step 5: Commit**

```bash
git add family-app/README.md
git commit -m "docs(family-app): README and manual verification checklist"
```

---

## Self-review (done when the plan was written)

**Spec coverage** (MVP table and screens of the family-app spec): open account (Task 4 `open_family_account`, Task 6 first screen); deposit (`deposit`, home); send/withdraw (`send_money`, send screen; over-limit becomes a request); add payee (`add_payee`, people); trusted people list and add (`add_guardian`, people); approve (`approve_request`, approvals); history (`load_history`, home); first launch with QR/6-digit code and "waiting to be added" (`pair_with_qr`/`pair_with_pin`, first screen, polling `list_vaults`); discovery (Task 1); Счёт, Перевод, Доверенные лица, Одобрения screens (Task 6). Gaps, all stated above with reasons: urgent add, camera scanning, mobile builds, the spec's 30 s refresh and its 15-call discovery.

**Placeholders:** none; every file is given in full and was compiled/run before it was written here.

**Type consistency:** command names and camelCase argument names in Task 4's table match `mock-api.js` handlers and the `call(...)` sites in Task 6 (`qrText`, `dailyLimit`, `vault`, `recipient`, `amount`, `nonce`, `decimals`); DTO field names in `dto.rs` match what the screens read (`permissions.send`, `remaining_today`, `payees[].active`, `pending[].nonce`).

**How this plan was checked:** the intermediate states each task commits were compiled separately (Task 2: 2 tests; Task 3: 26; Task 4: 31) in a copy of the real workspace, and the final UI passes the 6 unit tests and the 12-scenario smoke test. The real app was launched on Devnet and loaded its first screen. NOT verified: a physical kiosk, any transaction against a funded key, Android/Windows.
