# family-app-core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `family-app-core`, a headless Rust library with everything a future Tauri UI shell needs: local identity (keypair), on-chain vault discovery, every everyday admin-cli-equivalent action (open account, deposit, send, add payee, add payee urgently, add approver, approve withdrawal), and BLE pairing as the central role (scanning for and writing to a kiosk in first-run setup).

**Architecture:** Three layers, each independently testable: (1) pure instruction builders hoisted into the existing shared `family-vault-client` crate (a third consumer besides admin-cli and kiosk-gui now needs them, so a fourth duplication is worse than the hoist); (2) a new `tools/family-app-core` library crate wrapping those builders with RPC send/sign, local identity, and on-chain discovery; (3) a BLE central module in the same crate, reusing the wire format `family-vault-client::pairing` already shared with the kiosk (BLE peripheral). No UI, no Tauri, no IPC in this plan — that is a follow-up plan once this one lands, per the family-app design spec's own scope split.

**Tech Stack:** Rust, existing `family-vault-client`/`family-vault` crates, `solana-client =4.2.2` (RPC, matching the rest of this workspace's pins), `btleplug` (BLE central — version TBD by Task 5's own compile spike, see that task), TDD throughout.

**Spec:** `docs/superpowers/specs/2026-09-23-family-app-design.md` — sections "Vault Discovery", "MVP Scope", "Identity & Bootstrap". This plan also corrects one inaccuracy in that spec's "Vault Discovery" section (see Task 3).

## Global Constraints

- Work directly on `master` (established pattern for this repo). End every commit message with the line `Claude-Session: https://claude.ai/code/session_01VLZAgeertuGTkdt8LXpiEJ`.
- New crate `tools/family-app-core` and the touched parts of `tools/family-vault-client` both carry `[lints.rust] unsafe_code = "deny", dead_code = "deny"`.
- Two incompatible Solana `Pubkey` generations coexist: `anchor_lang::prelude::Pubkey` (Anchor bindings) and `solana_pubkey::Pubkey` (client side). Convert ONLY with `family_vault_client::{to_anchor, to_client}`.
- MVP explicitly excludes owner-rotation flows (`request-rotation`/`execute-rotation`/`cancel-rotation`) — do not build these; the family-app design spec lists them as out of scope.
- No process::exit / no CLI parsing in this crate — everything returns `Result`. This is a library other code (a future Tauri backend, or tests) calls into, not a binary.
- Never invent unverified crate APIs. Where this plan cites a function/type, it was checked against the pinned version's source in this environment (`~/.cargo/registry/src/...`) before being written down. Task 5 is the one exception (BLE central library not yet chosen/verified) and says so explicitly — its first step is a verification spike, not a leap of faith.
- Tests use real logic, not mocks. RPC-dependent tests that need a live endpoint are `#[ignore = "..."]` and documented as manual, matching the convention already used in `kiosk-gui/src/setup.rs` and `kiosk-gui/src/ble.rs`.

## File Structure

- Create `tools/family-vault-client/src/builders.rs` — pure instruction builders shared by admin-cli, kiosk-gui, and family-app-core.
- Modify `tools/family-vault-client/src/lib.rs` (`pub mod builders;`).
- Create `tools/family-app-core/Cargo.toml`, `tools/family-app-core/src/lib.rs`, `tools/family-app-core/src/identity.rs`, `tools/family-app-core/src/discovery.rs`, `tools/family-app-core/src/actions.rs`, `tools/family-app-core/src/pairing.rs`.
- Modify `Cargo.toml` (workspace members) and `docs/superpowers/specs/2026-09-23-family-app-design.md` (Task 3's spec correction).

---

### Task 1: Shared instruction builders in `family-vault-client`

**Files:**
- Create: `tools/family-vault-client/src/builders.rs`
- Modify: `tools/family-vault-client/src/lib.rs` (add `pub mod builders;`)

**Interfaces:**
- Consumes: `family_vault_client::{to_anchor, to_client, to_client_metas, vault_pda, beneficiary_list_pda, approvers_pda, request_pda}` (all already exist).
- Produces (module `family_vault_client::builders`, all `pub`, all return a plain `solana_instruction::Instruction` — no RPC, no signing):
  - `fn build_initialize_vault_instruction(program_id: &Pubkey, payer: &Pubkey, owner: &Pubkey, mint: &Pubkey, is_pool: bool, daily_limit: u64) -> Instruction`
  - `fn build_initialize_approvers_instruction(program_id: &Pubkey, payer: &Pubkey, vault: &Pubkey) -> Instruction`
  - `fn build_add_trusted_approver_instruction(program_id: &Pubkey, signer: &Pubkey, vault: &Pubkey, address: &Pubkey, existing_approver: Option<Pubkey>) -> Instruction`
  - `fn build_add_beneficiary_instruction(program_id: &Pubkey, signer: &Pubkey, vault: &Pubkey, name: &str, address: &Pubkey) -> Instruction`
  - `fn build_add_beneficiary_urgent_instruction(program_id: &Pubkey, approver_one: &Pubkey, approver_two: &Pubkey, vault: &Pubkey, name: &str, address: &Pubkey) -> Instruction`
  - `fn build_deposit_instruction(program_id: &Pubkey, depositor: &Pubkey, vault: &Pubkey, vault_token_account: &Pubkey, mint: &Pubkey, amount: u64) -> Instruction`
  - `fn build_withdraw_instruction(program_id: &Pubkey, signer: &Pubkey, vault: &Pubkey, vault_token_account: &Pubkey, mint: &Pubkey, recipient: &Pubkey, amount: u64) -> Instruction`
  - `fn build_request_withdrawal_instruction(program_id: &Pubkey, signer: &Pubkey, vault: &Pubkey, mint: &Pubkey, recipient: &Pubkey, amount: u64, nonce: u64) -> Instruction`
  - `fn build_approve_withdrawal_instruction(program_id: &Pubkey, signer: &Pubkey, vault: &Pubkey, vault_token_account: &Pubkey, mint: &Pubkey, recipient: &Pubkey, nonce: u64) -> Instruction`

All nine mirror `tools/admin-cli/src/main.rs`'s existing `cmd_*` functions exactly (same Anchor `accounts`/`instruction` structs, same account order/flags) — admin-cli built and devnet-tested each of these already; this task only separates "build the instruction" from "sign and send it", so admin-cli's own behavior does not change (a later, optional cleanup could have admin-cli call these builders too, but that is NOT part of this task — do not touch `tools/admin-cli`).

- [ ] **Step 1: Write the failing tests**

Create `tools/family-vault-client/src/builders.rs` containing ONLY this test module (the implementation comes in Step 3):

```rust
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
        assert!(ix.accounts.iter().any(|m| m.pubkey == key(2) && m.is_signer));
        assert!(ix.accounts.iter().any(|m| m.pubkey == key(3) && m.is_signer));
    }

    #[test]
    fn initialize_vault_derives_different_vaults_for_pool_and_personal() {
        let pool = build_initialize_vault_instruction(&program_id(), &key(2), &key(3), &key(4), true, 0);
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
        assert_eq!(ix.data, family_vault::instruction::InitializeApprovers {}.data());
        let (approvers, _) = approvers_pda(&program_id(), &vault);
        assert!(ix.accounts.iter().any(|m| m.pubkey == key(2) && m.is_signer));
        assert!(ix.accounts.iter().any(|m| m.pubkey == approvers));
    }

    #[test]
    fn add_trusted_approver_omits_existing_approver_when_none() {
        let vault = key(3);
        let ix = build_add_trusted_approver_instruction(&program_id(), &key(2), &vault, &key(9), None);
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
        assert!(ix.accounts.iter().any(|m| m.pubkey == key(5) && m.is_signer));
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
        assert!(ix.accounts.iter().any(|m| m.pubkey == key(6) && m.is_signer));
        assert!(ix.accounts.iter().any(|m| m.pubkey == key(7) && m.is_signer));
    }

    #[test]
    fn deposit_derives_the_depositor_token_account_from_the_mint() {
        let vault = key(3);
        let vault_token_account = key(10);
        let mint = key(4);
        let ix =
            build_deposit_instruction(&program_id(), &key(2), &vault, &vault_token_account, &mint, 500);
        assert_eq!(ix.data, family_vault::instruction::Deposit { amount: 500 }.data());
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
        assert_eq!(ix.data, family_vault::instruction::Withdraw { amount: 250 }.data());
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
        let a = build_request_withdrawal_instruction(&program_id(), &key(2), &vault, &mint, &recipient, 100, 0);
        let b = build_request_withdrawal_instruction(&program_id(), &key(2), &vault, &mint, &recipient, 100, 1);
        assert_eq!(
            a.data,
            family_vault::instruction::RequestWithdrawal { amount: 100, nonce: 0 }.data()
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
        assert_eq!(ix.data, family_vault::instruction::ApproveWithdrawal {}.data());
        let (pending, _) = request_pda(&program_id(), &vault, 7);
        let (approvers, _) = approvers_pda(&program_id(), &vault);
        assert!(ix.accounts.iter().any(|m| m.pubkey == pending));
        assert!(ix.accounts.iter().any(|m| m.pubkey == approvers));
        let recipient_ata =
            spl_associated_token_account::get_associated_token_address(&recipient, &mint);
        assert!(ix.accounts.iter().any(|m| m.pubkey == recipient_ata));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --package family-vault-client builders:: 2>&1 | tail -20`
Expected: compile FAIL — every `build_*` function name unresolved.

- [ ] **Step 3: Write the implementation**

Insert this at the TOP of `tools/family-vault-client/src/builders.rs`, above the test module:

```rust
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
    let vault_token_account = spl_associated_token_account::get_associated_token_address(&vault, mint);

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
        data: family_vault::instruction::InitializeVault { is_pool, daily_limit }.data(),
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
```

In `tools/family-vault-client/src/lib.rs`, add directly below `pub mod pairing;`:

```rust
pub mod builders;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --package family-vault-client 2>&1 | tail -20`
Expected: PASS — the existing tests plus 11 new ones in `builders::tests`.

- [ ] **Step 5: Lint and commit**

Run: `cargo clippy --package family-vault-client --all-targets 2>&1 | tail -10` — expected: clean.

```bash
git add tools/family-vault-client
git commit -m "feat(family-vault-client): hoist shared instruction builders

Nine pure build_*_instruction functions (open account, init approvers,
add/add-urgent beneficiary, add approver, deposit, withdraw,
request/approve withdrawal), mirroring admin-cli's existing on-chain
logic exactly. admin-cli itself is unchanged; family-app-core (next)
becomes the first consumer that doesn't duplicate this a third time.

Claude-Session: https://claude.ai/code/session_01VLZAgeertuGTkdt8LXpiEJ"
```

---

### Task 2: `family-app-core` crate scaffold + identity

**Files:**
- Create: `tools/family-app-core/Cargo.toml`, `tools/family-app-core/src/lib.rs`, `tools/family-app-core/src/identity.rs`
- Modify: `Cargo.toml` (workspace `members`)

**Interfaces:**
- Produces (module `family_app_core::identity`): `enum IdentityError { Io(std::io::Error) }` (`Debug`; `Display`; `std::error::Error`), `fn generate_or_load_identity(path: &std::path::Path) -> Result<solana_keypair::Keypair, IdentityError>`.

- [ ] **Step 1: Scaffold the crate**

Create `tools/family-app-core/Cargo.toml`:

```toml
[package]
name = "family-app-core"
version = "0.1.0"
edition = "2021"

[dependencies]
anchor-lang = "1.2.0"
family-vault = { version = "0.1.0", path = "../../programs/family-vault", features = ["cpi"] }
family-vault-client = { version = "0.1.0", path = "../family-vault-client" }
solana-account-decoder-client-types = { version = "=4.2.2", features = ["agave-unstable-api"] }
solana-client = "=4.2.2"
solana-instruction = "3.0.0"
solana-keypair = "3.0.1"
solana-pubkey = "=4.2.1"
solana-signature = "=3.4.1"
solana-signer = "3.0.0"
solana-transaction = { version = "4.1.6", features = ["wincode"] }
spl-associated-token-account = { version = "8.0.0", features = ["no-entrypoint"] }
spl-token = { version = "9.0.0", features = ["no-entrypoint"] }

[lints.rust]
unsafe_code = "deny"
dead_code = "deny"
```

Create `tools/family-app-core/src/lib.rs`:

```rust
//! Headless core for the family-facing companion app: local identity,
//! on-chain vault discovery, every everyday action (open an account,
//! deposit, send, add a payee, add a trusted person, approve an
//! over-limit withdrawal), and BLE pairing with a kiosk during its
//! first-run setup. No UI, no process arguments, no `std::process::exit`
//! anywhere — every public function returns a `Result` so a future Tauri
//! (or test) caller decides what to do with a failure.
//!
//! See `docs/superpowers/specs/2026-09-23-family-app-design.md`.

pub mod discovery;
pub mod identity;
```

In the workspace root `Cargo.toml`, add `"tools/family-app-core"` to the `members` array (alongside the existing entries).

- [ ] **Step 2: Write the failing tests**

Create `tools/family-app-core/src/identity.rs` containing ONLY this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use solana_signer::Signer;

    fn temp_path(name: &str) -> std::path::PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        std::env::temp_dir().join(format!("family-app-core-test-{name}-{nanos}.json"))
    }

    #[test]
    fn generates_a_new_identity_when_none_exists_on_disk() {
        let path = temp_path("fresh");
        assert!(!path.exists());
        let identity = generate_or_load_identity(&path).unwrap();
        assert!(path.exists(), "a keypair file must have been written");
        assert_ne!(identity.pubkey(), solana_pubkey::Pubkey::new_from_array([0u8; 32]));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn loads_the_same_identity_on_a_second_call() {
        let path = temp_path("stable");
        let first = generate_or_load_identity(&path).unwrap();
        let second = generate_or_load_identity(&path).unwrap();
        assert_eq!(first.pubkey(), second.pubkey());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_corrupt_keypair_file_is_reported_as_an_error_not_a_panic() {
        let path = temp_path("corrupt");
        std::fs::write(&path, b"not a keypair").unwrap();
        let err = generate_or_load_identity(&path).unwrap_err();
        assert!(matches!(err, IdentityError::Io(_)));
        std::fs::remove_file(&path).unwrap();
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --package family-app-core identity:: 2>&1 | tail -15`
Expected: compile FAIL — `generate_or_load_identity`/`IdentityError` not found.

- [ ] **Step 4: Write the implementation**

Insert at the TOP of `tools/family-app-core/src/identity.rs`:

```rust
//! Local identity: the app's own keypair, generated once on first launch
//! and reused after that. Same principle as the kiosk's own first-run
//! setup (`kiosk-gui/src/setup.rs::generate_or_load_owner_keypair`) — no
//! seed phrase is ever shown or exported, and nothing here talks to an
//! RPC endpoint.

use std::fmt;
use std::path::Path;

use solana_keypair::Keypair;

#[derive(Debug)]
pub enum IdentityError {
    Io(std::io::Error),
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentityError::Io(e) => write!(f, "could not access the identity keypair file: {e}"),
        }
    }
}

impl std::error::Error for IdentityError {}

/// Loads the identity at `path` if it exists, otherwise generates a fresh
/// one and persists it there (`solana_keypair::write_keypair_file` sets
/// `0600` permissions on unix automatically). Resumable: calling this
/// again after a crash mid-first-launch just loads what was already
/// written, exactly like the kiosk's own bootstrap.
pub fn generate_or_load_identity(path: &Path) -> Result<Keypair, IdentityError> {
    if path.exists() {
        return solana_keypair::read_keypair_file(path)
            .map_err(|e| IdentityError::Io(std::io::Error::other(e.to_string())));
    }
    let keypair = Keypair::new();
    solana_keypair::write_keypair_file(&keypair, path)
        .map_err(|e| IdentityError::Io(std::io::Error::other(e.to_string())))?;
    Ok(keypair)
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --package family-app-core 2>&1 | tail -15`
Expected: PASS — `3 passed`.

- [ ] **Step 6: Lint and commit**

Run: `cargo check --workspace 2>&1 | tail -10` (confirms the new workspace member doesn't break anything) then `cargo clippy --package family-app-core --all-targets 2>&1 | tail -10` — expected: clean.

```bash
git add tools/family-app-core Cargo.toml
git commit -m "feat(family-app-core): scaffold the crate and add local identity

generate_or_load_identity mirrors the kiosk's own first-run keypair
bootstrap: generate once, persist, reuse — no seed phrase ever shown.

Claude-Session: https://claude.ai/code/session_01VLZAgeertuGTkdt8LXpiEJ"
```

---

### Task 3: Vault discovery

**Files:**
- Create: `tools/family-app-core/src/discovery.rs`
- Modify: `tools/family-app-core/src/lib.rs` (add `pub mod discovery;` — already added in Task 2's scaffold; if not, add it now)
- Modify: `docs/superpowers/specs/2026-09-23-family-app-design.md` ("Vault Discovery" section)

**Interfaces:**
- Consumes: `family_vault_client::to_client`, `family_vault::state::{BeneficiaryList, TrustedApprovers, MAX_BENEFICIARIES, MAX_APPROVERS, BENEFICIARY_NAME_LEN}`.
- Produces (module `family_app_core::discovery`): `fn discover_my_vaults(rpc: &solana_client::rpc_client::RpcClient, program_id: &solana_pubkey::Pubkey, me: &solana_pubkey::Pubkey) -> Result<Vec<solana_pubkey::Pubkey>, String>`.

**Spec correction (do this first, it's one paragraph):** In `docs/superpowers/specs/2026-09-23-family-app-design.md`, find the "Vault Discovery" section's sentence: `"getProgramAccounts with memcmp filters is a single RPC call with no extra indexing infrastructure needed at MVP scale."` Replace it with:

```
`getProgramAccounts`'s filters are ANDed together within one call, so
checking "does any of up to 10 beneficiary slots / 5 approver slots equal
my pubkey" cannot be done in a single call — Solana has no OR filter.
MVP discovery instead issues one filtered `getProgramAccounts` call per
possible slot index (10 for `BeneficiaryList`, 5 for `TrustedApprovers`:
15 RPC calls total), each scoped by account size plus a `memcmp` at that
slot's exact byte offset, and merges/dedupes the results client-side. No
extra indexing infrastructure is still needed at MVP scale — 15 calls is
fine for an app that runs this on launch and on manual refresh, not in a
hot loop.
```

- [ ] **Step 1: Write the failing tests**

Create `tools/family-app-core/src/discovery.rs` containing ONLY this test module (offset-math tests first — pure, no RPC needed; the RPC-dependent test is added in Step 4 and is `#[ignore]`):

```rust
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

    // Requires a live RPC endpoint with real BeneficiaryList/TrustedApprovers
    // accounts to scan. Run manually with
    // `cargo test -p family-app-core discovery::tests::discovers_a_real_vault -- --ignored --nocapture`
    // after setting the constants below to a program id / pubkey known (from
    // devnet) to actually appear in at least one vault's beneficiary or
    // approver list.
    #[test]
    #[ignore = "needs a live RPC endpoint and a known devnet vault membership"]
    fn discovers_a_real_vault() {
        let rpc = solana_client::rpc_client::RpcClient::new(
            "https://api.devnet.solana.com".to_string(),
        );
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

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --package family-app-core discovery:: 2>&1 | tail -15`
Expected: compile FAIL — `beneficiary_address_offset`/`approver_offset` not found.

- [ ] **Step 3: Write the offset-math implementation and verify it passes**

Insert at the TOP of `tools/family-app-core/src/discovery.rs`, above the test module:

```rust
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
```

Run: `cargo test --package family-app-core discovery:: 2>&1 | tail -15`
Expected: PASS — `4 passed; 0 failed; 1 ignored`.

- [ ] **Step 4: Write the RPC-scanning implementation**

Append to `tools/family-app-core/src/discovery.rs`, below the offset functions and above the test module:

```rust
use std::collections::BTreeSet;

use anchor_lang::AccountDeserialize;
use family_vault::state::{BeneficiaryList, TrustedApprovers};
use solana_account_decoder_client_types::UiAccountEncoding;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};
use solana_pubkey::Pubkey;

fn scan_for_pubkey_at_offset(
    rpc: &RpcClient,
    program_id: &Pubkey,
    data_size: u64,
    offset: usize,
    needle: &Pubkey,
) -> Result<Vec<Vec<u8>>, String> {
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![
            RpcFilterType::DataSize(data_size),
            RpcFilterType::Memcmp(Memcmp::new(
                offset,
                MemcmpEncodedBytes::Bytes(needle.to_bytes().to_vec()),
            )),
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
        .map_err(|e| format!("getProgramAccounts at offset {offset}: {e}"))?;

    accounts
        .into_iter()
        .map(|(_pubkey, ui_account)| {
            ui_account
                .to_account()
                .map(|account| account.data)
                .ok_or_else(|| "could not decode account data".to_string())
        })
        .collect()
}

/// Every vault where `me` is currently a payee.
fn vaults_where_i_am_a_beneficiary(
    rpc: &RpcClient,
    program_id: &Pubkey,
    me: &Pubkey,
) -> Result<BTreeSet<Pubkey>, String> {
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
                .map_err(|e| format!("decoding BeneficiaryList: {e}"))?;
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
) -> Result<BTreeSet<Pubkey>, String> {
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
                .map_err(|e| format!("decoding TrustedApprovers: {e}"))?;
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
) -> Result<Vec<Pubkey>, String> {
    let mut vaults = vaults_where_i_am_a_beneficiary(rpc, program_id, me)?;
    vaults.extend(vaults_where_i_am_an_approver(rpc, program_id, me)?);
    Ok(vaults.into_iter().collect())
}
```

- [ ] **Step 5: Run to verify everything still passes**

Run: `cargo test --package family-app-core 2>&1 | tail -15`
Expected: PASS — offset tests still `4 passed`, `1 ignored`, identity tests unaffected.

- [ ] **Step 6: Lint and commit**

Run: `cargo clippy --workspace --all-targets 2>&1 | tail -10` — expected: clean.

```bash
git add tools/family-app-core docs/superpowers/specs/2026-09-23-family-app-design.md
git commit -m "feat(family-app-core): discover vaults by scanning beneficiary/approver slots

getProgramAccounts filters are ANDed, not ORed, so checking membership
across up to 10 payee / 5 approver slots needs one filtered call per
slot (15 total), not the single call the spec previously claimed —
corrected that sentence. Offset math is unit-tested without an RPC; the
scan itself is exercised by one --ignored live test.

Claude-Session: https://claude.ai/code/session_01VLZAgeertuGTkdt8LXpiEJ"
```

---

### Task 4: Everyday actions (send/sign wrappers around Task 1's builders)

**Files:**
- Create: `tools/family-app-core/src/actions.rs`
- Modify: `tools/family-app-core/src/lib.rs` (add `pub mod actions;`)

**Interfaces:**
- Consumes: `family_vault_client::builders::*` (Task 1), `family_vault_client::{to_client, fetch_mint_decimals}`, `family_vault::state::{Vault, PendingWithdrawal}`.
- Produces (module `family_app_core::actions`):
  - `enum ActionError { NoConnection, Rejected(String) }` (`Debug`, `Display`, `std::error::Error`)
  - `fn send_instructions(rpc: &RpcClient, payer: &Keypair, instructions: &[Instruction], extra_signers: &[&Keypair]) -> Result<solana_signature::Signature, ActionError>`
  - `fn open_pool_vault(rpc: &RpcClient, program_id: &Pubkey, owner: &Keypair, mint: &Pubkey, daily_limit: u64) -> Result<Pubkey, ActionError>`
  - `fn deposit(rpc: &RpcClient, program_id: &Pubkey, depositor: &Keypair, vault: &Pubkey, amount_whole_units: u64) -> Result<solana_signature::Signature, ActionError>`
  - `fn send(rpc: &RpcClient, program_id: &Pubkey, owner: &Keypair, vault: &Pubkey, recipient: &Pubkey, amount_whole_units: u64) -> Result<solana_signature::Signature, ActionError>`
  - `fn add_beneficiary(rpc: &RpcClient, program_id: &Pubkey, owner: &Keypair, vault: &Pubkey, name: &str, address: &Pubkey) -> Result<solana_signature::Signature, ActionError>`
  - `fn add_beneficiary_urgent(rpc: &RpcClient, program_id: &Pubkey, approver_one: &Keypair, approver_two: &Keypair, vault: &Pubkey, name: &str, address: &Pubkey) -> Result<solana_signature::Signature, ActionError>`
  - `fn add_trusted_approver(rpc: &RpcClient, program_id: &Pubkey, owner: &Keypair, vault: &Pubkey, address: &Pubkey, existing_approver: Option<&Keypair>) -> Result<solana_signature::Signature, ActionError>`
  - `fn approve_withdrawal(rpc: &RpcClient, program_id: &Pubkey, signer: &Keypair, vault: &Pubkey, nonce: u64) -> Result<solana_signature::Signature, ActionError>`

- [ ] **Step 1: Write the failing tests**

Create `tools/family-app-core/src/actions.rs` containing ONLY this test module (real logic that doesn't need an RPC — pure error-mapping and instruction-shape checks; the RPC-dependent paths are exercised the same way this workspace already tests them, via manual devnet runs, not unit tests, because they need a funded keypair):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_io_errors_to_no_connection() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        let client_err: solana_client::client_error::ClientError = io_err.into();
        assert!(matches!(map_client_error(client_err), ActionError::NoConnection));
    }

    #[test]
    fn maps_other_errors_to_rejected() {
        let client_err: solana_client::client_error::ClientError =
            solana_client::client_error::ClientErrorKind::Custom("nope".to_string()).into();
        assert!(matches!(map_client_error(client_err), ActionError::Rejected(_)));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --package family-app-core actions:: 2>&1 | tail -15`
Expected: compile FAIL — `map_client_error`/`ActionError` not found.

- [ ] **Step 3: Write the implementation**

Insert at the TOP of `tools/family-app-core/src/actions.rs`, above the test module:

```rust
//! Everyday bank-style actions, each a thin RPC-send/sign wrapper around
//! one of `family_vault_client::builders`' pure instruction builders.
//! Mirrors `admin-cli`'s command set exactly (see that tool's `cmd_*`
//! functions) but returns `Result` instead of printing and calling
//! `std::process::exit` — this is a library a future UI calls into, not a
//! CLI. Owner-rotation actions are out of scope for the MVP (see the
//! family-app design spec) and are not here.

use std::fmt;

use anchor_lang::AccountDeserialize;
use family_vault::state::{PendingWithdrawal, Vault};
use family_vault_client::builders::{
    build_add_beneficiary_instruction, build_add_beneficiary_urgent_instruction,
    build_add_trusted_approver_instruction, build_approve_withdrawal_instruction,
    build_deposit_instruction, build_initialize_approvers_instruction,
    build_initialize_vault_instruction, build_request_withdrawal_instruction,
    build_withdraw_instruction,
};
use family_vault_client::{fetch_mint_decimals, to_client, vault_pda};
use solana_client::rpc_client::RpcClient;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[derive(Debug)]
pub enum ActionError {
    /// RPC endpoint unreachable.
    NoConnection,
    /// RPC responded but the program/transaction rejected the request.
    Rejected(String),
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionError::NoConnection => write!(f, "no connection to RPC endpoint"),
            ActionError::Rejected(msg) => write!(f, "rejected: {msg}"),
        }
    }
}

impl std::error::Error for ActionError {}

fn map_client_error(err: solana_client::client_error::ClientError) -> ActionError {
    match err.kind() {
        solana_client::client_error::ClientErrorKind::Io(_)
        | solana_client::client_error::ClientErrorKind::Reqwest(_) => ActionError::NoConnection,
        other => ActionError::Rejected(other.to_string()),
    }
}

/// Sends one or more instructions in a single transaction, signed by
/// `payer` plus any `extra_signers` (e.g. a second trusted approver for
/// `add_beneficiary_urgent`, or an existing approver co-signing a new
/// one).
pub fn send_instructions(
    rpc: &RpcClient,
    payer: &Keypair,
    instructions: &[Instruction],
    extra_signers: &[&Keypair],
) -> Result<solana_signature::Signature, ActionError> {
    let blockhash = rpc.get_latest_blockhash().map_err(map_client_error)?;
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra_signers);
    let tx = solana_transaction::Transaction::new_signed_with_payer(
        instructions,
        Some(&payer.pubkey()),
        &signers,
        blockhash,
    );
    rpc.send_and_confirm_transaction(&tx).map_err(map_client_error)
}

fn fetch_vault(rpc: &RpcClient, vault: &Pubkey) -> Result<Vault, ActionError> {
    let bytes = rpc.get_account_data(vault).map_err(map_client_error)?;
    Vault::try_deserialize(&mut &bytes[..]).map_err(|e| ActionError::Rejected(e.to_string()))
}

fn fetch_vault_token_account_and_mint(
    rpc: &RpcClient,
    vault: &Pubkey,
) -> Result<(Pubkey, Pubkey), ActionError> {
    use spl_token::solana_program::program_pack::Pack;
    let vault_data = fetch_vault(rpc, vault)?;
    let vault_token_account = to_client(vault_data.vault_token_account);
    let vta_bytes = rpc
        .get_account_data(&vault_token_account)
        .map_err(map_client_error)?;
    let mint = spl_token::state::Account::unpack(&vta_bytes)
        .map_err(|e| ActionError::Rejected(e.to_string()))?
        .mint;
    Ok((vault_token_account, mint))
}

/// Opens a pool (family) vault for `owner`, then initializes its
/// approvers list — the two steps a family always needs before adding
/// anyone. Returns the new vault's address. Adding the first approvers
/// and payees is a separate call to `add_trusted_approver`/
/// `add_beneficiary` per person, same as `admin-cli`.
pub fn open_pool_vault(
    rpc: &RpcClient,
    program_id: &Pubkey,
    owner: &Keypair,
    mint: &Pubkey,
    daily_limit: u64,
) -> Result<Pubkey, ActionError> {
    let (vault, _) = vault_pda(program_id, &owner.pubkey(), true);
    let init_vault = build_initialize_vault_instruction(
        program_id,
        &owner.pubkey(),
        &owner.pubkey(),
        mint,
        true,
        daily_limit,
    );
    send_instructions(rpc, owner, &[init_vault], &[])?;

    let init_approvers = build_initialize_approvers_instruction(program_id, &owner.pubkey(), &vault);
    send_instructions(rpc, owner, &[init_approvers], &[])?;

    Ok(vault)
}

pub fn deposit(
    rpc: &RpcClient,
    program_id: &Pubkey,
    depositor: &Keypair,
    vault: &Pubkey,
    amount_whole_units: u64,
) -> Result<solana_signature::Signature, ActionError> {
    let (vault_token_account, mint) = fetch_vault_token_account_and_mint(rpc, vault)?;
    let decimals = fetch_mint_decimals(rpc, &mint).map_err(ActionError::Rejected)?;
    let amount = amount_whole_units * 10u64.pow(decimals);

    let ix = build_deposit_instruction(
        program_id,
        &depositor.pubkey(),
        vault,
        &vault_token_account,
        &mint,
        amount,
    );
    send_instructions(rpc, depositor, &[ix], &[])
}

/// The recipient must already be an approved payee — the program enforces
/// this regardless of what this function sends.
pub fn send(
    rpc: &RpcClient,
    program_id: &Pubkey,
    owner: &Keypair,
    vault: &Pubkey,
    recipient: &Pubkey,
    amount_whole_units: u64,
) -> Result<solana_signature::Signature, ActionError> {
    let (vault_token_account, mint) = fetch_vault_token_account_and_mint(rpc, vault)?;
    let decimals = fetch_mint_decimals(rpc, &mint).map_err(ActionError::Rejected)?;
    let amount = amount_whole_units * 10u64.pow(decimals);

    let ix = build_withdraw_instruction(
        program_id,
        &owner.pubkey(),
        vault,
        &vault_token_account,
        &mint,
        recipient,
        amount,
    );
    send_instructions(rpc, owner, &[ix], &[])
}

pub fn add_beneficiary(
    rpc: &RpcClient,
    program_id: &Pubkey,
    owner: &Keypair,
    vault: &Pubkey,
    name: &str,
    address: &Pubkey,
) -> Result<solana_signature::Signature, ActionError> {
    let ix = build_add_beneficiary_instruction(program_id, &owner.pubkey(), vault, name, address);
    send_instructions(rpc, owner, &[ix], &[])
}

/// Two distinct trusted approvers, not the owner — skips the normal 48h
/// cooldown. See `family_vault::instructions::add_beneficiary_urgent`.
pub fn add_beneficiary_urgent(
    rpc: &RpcClient,
    program_id: &Pubkey,
    approver_one: &Keypair,
    approver_two: &Keypair,
    vault: &Pubkey,
    name: &str,
    address: &Pubkey,
) -> Result<solana_signature::Signature, ActionError> {
    let ix = build_add_beneficiary_urgent_instruction(
        program_id,
        &approver_one.pubkey(),
        &approver_two.pubkey(),
        vault,
        name,
        address,
    );
    send_instructions(rpc, approver_one, &[ix], &[approver_two])
}

/// `existing_approver` is required (must co-sign) when adding a
/// second-or-later approver to a *personal* vault; a pool vault, and the
/// very first approver ever added, need only the owner. Passing `None`
/// when one is actually required surfaces as a normal `ActionError::
/// Rejected` from the program, not a panic.
pub fn add_trusted_approver(
    rpc: &RpcClient,
    program_id: &Pubkey,
    owner: &Keypair,
    vault: &Pubkey,
    address: &Pubkey,
    existing_approver: Option<&Keypair>,
) -> Result<solana_signature::Signature, ActionError> {
    let ix = build_add_trusted_approver_instruction(
        program_id,
        &owner.pubkey(),
        vault,
        address,
        existing_approver.map(|k| k.pubkey()),
    );
    let extra: Vec<&Keypair> = existing_approver.into_iter().collect();
    send_instructions(rpc, owner, &[ix], &extra)
}

/// Approves a pending over-limit withdrawal by nonce. Reads the pending
/// request's recipient and the vault's token/mint from chain first (the
/// caller only needs to remember the nonce), matching `admin-cli`'s own
/// `approve-withdrawal` behavior.
pub fn approve_withdrawal(
    rpc: &RpcClient,
    program_id: &Pubkey,
    signer: &Keypair,
    vault: &Pubkey,
    nonce: u64,
) -> Result<solana_signature::Signature, ActionError> {
    let (pending_withdrawal, _) = family_vault_client::request_pda(program_id, vault, nonce);
    let pw_bytes = rpc
        .get_account_data(&pending_withdrawal)
        .map_err(map_client_error)?;
    let pending = PendingWithdrawal::try_deserialize(&mut &pw_bytes[..])
        .map_err(|e| ActionError::Rejected(e.to_string()))?;
    let recipient = to_client(pending.to);

    let (vault_token_account, mint) = fetch_vault_token_account_and_mint(rpc, vault)?;

    let ix = build_approve_withdrawal_instruction(
        program_id,
        &signer.pubkey(),
        vault,
        &vault_token_account,
        &mint,
        &recipient,
        nonce,
    );
    send_instructions(rpc, signer, &[ix], &[])
}

/// Requests an over-limit withdrawal from a pool vault, to be later
/// confirmed with `approve_withdrawal`. Not part of the MVP's default
/// "send" flow (that goes straight through `send`/`Withdraw` for
/// within-limit amounts) but needed once a pool vault's daily limit is
/// exceeded.
#[allow(dead_code)] // wired into a UI action in a later plan
pub fn request_withdrawal(
    rpc: &RpcClient,
    program_id: &Pubkey,
    signer: &Keypair,
    vault: &Pubkey,
    recipient: &Pubkey,
    amount_whole_units: u64,
    nonce: u64,
) -> Result<solana_signature::Signature, ActionError> {
    let (_vault_token_account, mint) = fetch_vault_token_account_and_mint(rpc, vault)?;
    let decimals = fetch_mint_decimals(rpc, &mint).map_err(ActionError::Rejected)?;
    let amount = amount_whole_units * 10u64.pow(decimals);

    let ix = build_request_withdrawal_instruction(
        program_id,
        &signer.pubkey(),
        vault,
        &mint,
        recipient,
        amount,
        nonce,
    );
    send_instructions(rpc, signer, &[ix], &[])
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --package family-app-core 2>&1 | tail -15`
Expected: PASS — `actions::tests` `2 passed`, all earlier tests unaffected.

In `tools/family-app-core/src/lib.rs`, add:

```rust
pub mod actions;
```

- [ ] **Step 5: Full workspace check and commit**

Run: `cargo check --workspace 2>&1 | tail -15` then `cargo clippy --workspace --all-targets 2>&1 | tail -10` — expected: clean. (`request_withdrawal`'s `#[allow(dead_code)]` is intentional — remove it in the plan that wires up UI actions, once something calls it.)

```bash
git add tools/family-app-core
git commit -m "feat(family-app-core): everyday actions on top of the shared builders

open_pool_vault, deposit, send, add_beneficiary(_urgent),
add_trusted_approver, approve_withdrawal, request_withdrawal — each a
thin RPC-send/sign wrapper around family-vault-client::builders,
returning Result instead of admin-cli's process::exit style.

Claude-Session: https://claude.ai/code/session_01VLZAgeertuGTkdt8LXpiEJ"
```

---

### Task 5: BLE pairing (central role)

**Files:**
- Create: `tools/family-app-core/src/pairing.rs`
- Modify: `tools/family-app-core/Cargo.toml` (BLE dependency, pinned only after Step 1's spike), `tools/family-app-core/src/lib.rs` (add `pub mod pairing;`)

**Interfaces:**
- Consumes: `family_vault_client::pairing::{encode_frame, decode_qr_payload, PairingRequest, SERVICE_UUID_U128, PAIRING_CHARACTERISTIC_UUID_U128}` (already exist, shared with the kiosk peripheral side).
- Produces (module `family_app_core::pairing`): `enum PairingError { ... }` (exact variants decided in Step 3, once the real BLE crate's error surface is known — do not guess it before that), `fn pair_with_kiosk(qr_text: &str, name: &str, my_pubkey: solana_pubkey::Pubkey) -> Result<(), PairingError>` — blocks the calling thread; scans for the kiosk's advertised service, connects, writes the encoded frame WITH response (so a wrong/expired code surfaces as an error rather than silently vanishing — required by the family-app spec's "Identity & Bootstrap" section), and returns once the write is acknowledged.

**Unlike every other task in this plan, this task's BLE crate and exact API were NOT verified against source before this plan was written** (the kiosk's peripheral side used `bluer`, which was verified with a real compile spike before that plan was written; the family-app design spec names `btleplug` for the central role, but nobody has compiled it in this workspace yet). Step 1 is that verification — do not skip it or guess the API from memory.

- [ ] **Step 1: Verify the BLE central crate compiles and learn its real API**

In your own scratch directory (NOT this repo — e.g. your assigned scratchpad), run `cargo new --quiet blespike && cd blespike`, then try the current stable `btleplug` release: add it to `Cargo.toml` (`cargo add btleplug tokio --features tokio/full` or by hand), and write a minimal `main.rs` that: creates a `Manager`, gets the first central `Adapter`, starts a scan filtered (if the API supports it) or unfiltered, waits a few seconds, lists discovered peripherals' advertised service UUIDs, and — if any peripheral advertises `Uuid::from_u128(0x8f1a0000_5b1e_4c6a_9a7e_0d2f3a1b4c5d)` (the kiosk's `SERVICE_UUID_U128`) — connects, discovers services, finds the characteristic `Uuid::from_u128(0x8f1a0001_5b1e_4c6a_9a7e_0d2f3a1b4c5d)`, and writes a few test bytes to it with `WriteType::WithResponse`. Run `cargo build` and, if you have a way to actually exercise it against a running kiosk (or at least confirm the scan/connect calls compile and return the types you expect), do so.

Record in your task report: the exact `btleplug` version that compiled, the exact import paths and function/method signatures you used for: creating a `Manager`, getting adapters, starting/stopping a scan, listing/filtering discovered peripherals by advertised service UUID, connecting, discovering services/characteristics, and writing with response (including the write-with-response variant's exact name — earlier tasks in this project found that BLE libraries are easy to get subtly wrong on this exact point, e.g. `bluer`'s `write_without_response` boolean flag was initially set backwards in the kiosk's own peripheral code and caught only by review). If `btleplug`'s central API cannot filter a scan by service UUID directly, note that you'll filter client-side over each discovered peripheral's advertisement data instead.

If `btleplug` turns out not to compile cleanly in this workspace (dependency conflict, platform issue, etc.), STOP and report BLOCKED with what you tried — do not silently switch to a different crate without checking with the controller first, since the design spec specifically named `btleplug` for cross-platform reasons (Windows/Linux/Android) that a substitute might not share.

- [ ] **Step 2: Add the verified dependency**

Once Step 1 confirms a working version, add it to `tools/family-app-core/Cargo.toml`'s `[dependencies]` using the EXACT version and features you verified (not a caret/latest range — pin it, matching this workspace's convention of exact pins for every Solana/BLE dependency). Add a `tokio` dependency with whatever exact features your spike needed (likely `rt`, `rt-multi-thread` or similar — use what you verified, not a guess).

- [ ] **Step 3: Write the failing tests**

Create `tools/family-app-core/src/pairing.rs` containing ONLY this test module (pure logic first — the actual scan/connect/write path needs a real kiosk in range and is `#[ignore]`, matching this project's established pattern for hardware-dependent tests in `kiosk-gui/src/ble.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use family_vault_client::pairing::decode_qr_payload;
    use solana_pubkey::Pubkey;

    #[test]
    fn rejects_qr_text_that_does_not_decode() {
        let err = pair_with_kiosk("not a real pairing code", "Внук Саша", Pubkey::new_from_array([1u8; 32]));
        assert!(matches!(err, Err(PairingError::BadQrText)));
    }

    #[test]
    fn a_valid_qr_payload_decodes_before_any_bluetooth_activity() {
        // Sanity check that this test's own fixture round-trips through
        // the SAME decoder pair_with_kiosk uses internally, so the
        // "rejects bad text" test above is exercising the real check.
        let secret = [7u8; family_vault_client::pairing::AUTH_LEN];
        let text = family_vault_client::pairing::encode_qr_payload(&secret);
        assert_eq!(decode_qr_payload(&text), Some(secret));
    }

    // Requires a real kiosk in first-run pairing mode, in Bluetooth range,
    // showing a QR — scan it with a phone or `zbarimg`/similar to get the
    // text. Run manually with
    // `PAIR_QR='fvpair1:...' cargo test -p family-app-core pairing::tests::pairs_with_a_real_kiosk -- --ignored --nocapture`
    #[test]
    #[ignore = "needs a real kiosk in range, in first-run pairing mode"]
    fn pairs_with_a_real_kiosk() {
        let qr_text = std::env::var("PAIR_QR").expect("set PAIR_QR to the kiosk's scanned QR text");
        let my_pubkey = Pubkey::new_unique();
        pair_with_kiosk(&qr_text, "Тестовый Аккаунт", my_pubkey).unwrap();
    }
}
```

- [ ] **Step 4: Run to verify the two non-ignored tests fail**

Run: `cargo test --package family-app-core pairing:: 2>&1 | tail -15`
Expected: compile FAIL — `pair_with_kiosk`/`PairingError` not found.

- [ ] **Step 5: Write the implementation using Step 1's verified API**

Write `tools/family-app-core/src/pairing.rs`'s implementation (above the test module) using the EXACT types/methods/imports your Step 1 spike verified — do not reintroduce guessed API names here. At minimum it must:

1. Define `PairingError` with (at least) these variants, extended with whatever your verified crate's own error type needs to be wrapped: `BadQrText` (the QR text didn't decode via `family_vault_client::pairing::decode_qr_payload`), `NoAdapter`, `KioskNotFound` (scan timed out without finding a peripheral advertising `SERVICE_UUID_U128`), `ConnectFailed(String)`, `WriteRejected(String)` (the write-with-response came back as an error — e.g. wrong/expired code), `Ble(String)` (catch-all for the underlying crate's own error type via `Display`).
2. `pair_with_kiosk` first calls `decode_qr_payload(qr_text)`, returning `Err(PairingError::BadQrText)` immediately (no Bluetooth activity at all) if it fails — this is exactly what `rejects_qr_text_that_does_not_decode` checks.
3. Builds the frame with `family_vault_client::pairing::{PairingRequest, encode_frame}` from the decoded secret, `my_pubkey`, and `name`, propagating `encode_frame`'s `FrameError` (e.g. a too-long name) as `PairingError::Ble(...)` or a dedicated variant — your call, but it must not panic on a bad name.
4. Scans, finds the kiosk by its advertised `SERVICE_UUID_U128`, connects, discovers the `PAIRING_CHARACTERISTIC_UUID_U128` characteristic, and writes the encoded frame **with response** (never write-without-response — a wrong/expired code must come back as an error, per the family-app design spec's explicit requirement that family-app confirm registration rather than trust a bare acknowledgement). A write error becomes `PairingError::WriteRejected`.
5. Never logs the QR secret, the frame bytes, or `my_pubkey`'s private key material (logging the public key itself is fine).

- [ ] **Step 6: Run to verify everything passes**

Run: `cargo test --package family-app-core 2>&1 | tail -20`
Expected: PASS — new pairing tests `2 passed; 1 ignored`, everything else unaffected.

In `tools/family-app-core/src/lib.rs`, add:

```rust
pub mod pairing;
```

- [ ] **Step 7: Full workspace check and commit**

Run: `cargo check --workspace 2>&1 | tail -15`, `cargo clippy --workspace --all-targets 2>&1 | tail -15` — expected clean. If the BLE crate needed a specific async runtime setup (e.g. blocking on a `tokio` runtime the same way `kiosk-gui/src/ble.rs::wait_for_pairing` does with `tokio::runtime::Builder::new_current_thread()`), make sure `pair_with_kiosk` still presents a plain blocking `fn` to its caller — a future Tauri command can run it on its own thread; this crate has no opinion about that.

```bash
git add tools/family-app-core
git commit -m "feat(family-app-core): pair with a kiosk over BLE as the central

Scans for the kiosk's advertised pairing service, connects, and writes
the shared frame WITH response, so a wrong or expired code surfaces as
an error instead of a silent no-op. Reuses family-vault-client::pairing
(the same wire format the kiosk peripheral already speaks).

Claude-Session: https://claude.ai/code/session_01VLZAgeertuGTkdt8LXpiEJ"
```

---

## Final verification (after all 5 tasks)

Run the full workspace suite once more end to end: `cargo test --workspace 2>&1 | grep -E "test result|FAILED"` (if `programs/family-vault`'s integration tests fail with a missing `.so`, run `cargo build-sbf` inside `programs/family-vault` first — a known, unrelated environment quirk seen repeatedly in this project, not a defect in this plan's work) and `cargo clippy --workspace --all-targets 2>&1 | tail -10`.

## Explicitly not in this plan (future work)

- **Transaction history** (the design spec's MVP table row "История операций: read-only, decode past instructions/events for this vault"). Not built here — it needs `get_signatures_for_address` + `get_transaction` + decoding each of this program's instruction types from raw transaction data, which is a real chunk of work on its own (closer in size to Task 3 than a one-liner) and nothing else in this plan depends on it. Add it as its own task in a follow-up plan before calling family-app-core MVP-complete.
- The Tauri shell, IPC commands, and every UI screen — a separate plan, once this one is reviewed and merged, per the family-app design spec's own architecture split (Rust core vs. Tauri shell).
- `admin-cli` migrating to call `family_vault_client::builders` instead of its own inline copies — optional cleanup, not required for family-app to work.
- Owner-rotation actions (`request`/`cancel`/`execute_owner_rotation`) — explicitly out of scope for the family-app MVP per the design spec.
- Real end-to-end verification of Task 5 against a physical kiosk — needs a phone-scannable QR from a real device in first-run setup and is called out as manual in that task.
