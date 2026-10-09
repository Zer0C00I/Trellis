//! Everyday bank-style actions, each a thin RPC-send/sign wrapper around
//! one of `family_vault_client::builders`' pure instruction builders.
//! Covers the same everyday commands as `admin-cli` (see that tool's
//! `cmd_*` functions), minus owner rotation, but returns `Result` instead
//! of printing and calling `std::process::exit` — this is a library a
//! future UI calls into, not a CLI. Owner-rotation actions are out of
//! scope for the MVP (see the family-app design spec) and are not here.
//!
//! Amounts are always in the token's base units (`u64`). A UI reads
//! `decimals` from `vault_view::VaultView` and converts what the user typed
//! with `family_vault_client::parse_decimal_amount`.

use crate::error::map_client_error;
pub use crate::error::ActionError;
use crate::vault_view::next_free_nonce;
use anchor_lang::AccountDeserialize;
use family_vault::state::{PendingWithdrawal, Vault};
use family_vault_client::builders::{
    build_add_beneficiary_instruction, build_add_beneficiary_urgent_instruction,
    build_add_trusted_approver_instruction, build_approve_withdrawal_instruction,
    build_deposit_instruction, build_initialize_approvers_instruction,
    build_initialize_vault_instruction, build_request_withdrawal_instruction,
    build_withdraw_instruction,
};
use family_vault_client::{to_client, vault_pda};
use solana_client::rpc_client::RpcClient;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;
use solana_transaction_status_client_types::{TransactionConfirmationStatus, TransactionStatus};
use std::time::{Duration, Instant};

/// A zero amount is never a meaningful transfer; refuse it before any RPC.
fn require_nonzero(amount_base_units: u64) -> Result<(), ActionError> {
    if amount_base_units == 0 {
        return Err(ActionError::Rejected(
            "amount must be greater than zero".to_string(),
        ));
    }
    Ok(())
}

/// How long `send_instructions` polls for a verdict before giving up with
/// `Unconfirmed`. A blockhash is valid for roughly 60-90s.
const CONFIRM_DEADLINE: Duration = Duration::from_secs(60);
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// What one look at a signature's status means for the polling loop.
enum Poll {
    /// Not seen yet, or only `processed` (could still be dropped).
    Pending,
    /// The verdict: `Ok` = confirmed/finalized, `Err(Rejected)` = failed on chain.
    Done(Result<(), ActionError>),
}

fn classify_status(status: Option<TransactionStatus>) -> Poll {
    let Some(status) = status else {
        return Poll::Pending;
    };
    if let Some(err) = status.err {
        // The transaction executed and failed: it definitely did not take effect.
        return Poll::Done(Err(ActionError::Rejected(err.to_string())));
    }
    let confirmed = match status.confirmation_status {
        Some(
            TransactionConfirmationStatus::Confirmed | TransactionConfirmationStatus::Finalized,
        ) => true,
        Some(TransactionConfirmationStatus::Processed) => false,
        // Older nodes omit confirmation_status; `None` confirmations means rooted.
        None => status.confirmations.is_none_or(|c| c > 1),
    };
    if confirmed {
        Poll::Done(Ok(()))
    } else {
        Poll::Pending
    }
}

/// Maps an error from `send_transaction`: only a connection-level failure
/// leaves the outcome unknown; anything else is the node refusing it.
fn send_error(err: solana_client::client_error::ClientError, signature: Signature) -> ActionError {
    match map_client_error(err) {
        ActionError::NoConnection => ActionError::Unconfirmed(signature),
        other => other,
    }
}

/// Sends one or more instructions in a single transaction, signed by
/// `payer` plus any `extra_signers` (e.g. a second trusted approver for
/// `add_beneficiary_urgent`, or an existing approver co-signing a new
/// one).
///
/// The transaction is signed once, sent, and then its signature is polled
/// for up to ~60s (or until its blockhash expires). Retry rule for callers:
///
/// - `Ok(signature)`: confirmed on chain. Never resend.
/// - `Rejected` / `NoConnection`: the transaction was NOT accepted (the node
///   refused it, it failed on chain, or no connection could be made before
///   signing), so building a fresh transaction and retrying is safe.
/// - `Unconfirmed(signature)`: it was sent (or the connection dropped while
///   sending) and the outcome is unknown. Do
///   NOT retry with a new transaction (new blockhash, would pay twice).
///   Call [`confirm_signature`] first; only `Failed`, or `NotFound` after the
///   transaction's blockhash has expired (about 90s after sending), makes a
///   retry safe.
pub fn send_instructions(
    rpc: &RpcClient,
    payer: &Keypair,
    instructions: &[Instruction],
    extra_signers: &[&Keypair],
) -> Result<Signature, ActionError> {
    let (blockhash, last_valid_block_height) = rpc
        .get_latest_blockhash_with_commitment(rpc.commitment())
        .map_err(map_client_error)?;
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra_signers);
    let tx = solana_transaction::Transaction::new_signed_with_payer(
        instructions,
        Some(&payer.pubkey()),
        &signers,
        blockhash,
    );
    // The signature is known before sending, so a lost response cannot hide it.
    let signature = tx.signatures[0];
    let signature = match rpc.send_transaction(&tx) {
        Ok(returned) => returned,
        // A node rejection (preflight failure) means the transaction was not
        // accepted. A connection error might have struck after the request
        // arrived, so the transaction may still land: outcome unknown.
        Err(e) => return Err(send_error(e, signature)),
    };

    let deadline = Instant::now() + CONFIRM_DEADLINE;
    loop {
        // Any failure to poll leaves the outcome unknown, whatever its kind.
        let response = rpc
            .get_signature_statuses(&[signature])
            .map_err(|_| ActionError::Unconfirmed(signature))?;
        match classify_status(response.value.into_iter().next().flatten()) {
            Poll::Done(result) => return result.map(|()| signature),
            Poll::Pending => {}
        }
        if Instant::now() >= deadline {
            return Err(ActionError::Unconfirmed(signature));
        }
        // Stop early once the blockhash has expired; a failed height lookup
        // just means "keep polling until the deadline".
        if rpc
            .get_block_height()
            .is_ok_and(|height| height > last_valid_block_height)
        {
            return Err(ActionError::Unconfirmed(signature));
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// What the network knows about a signature, for resolving an
/// `ActionError::Unconfirmed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureOutcome {
    /// Landed and confirmed or finalized without error.
    Confirmed,
    /// Landed and failed (the transaction error text); it had no effect.
    Failed(String),
    /// This node has no record of it. It may still land until its blockhash
    /// expires (about 90s after sending); only then is a retry safe.
    NotFound,
}

/// Looks a signature up (including ledger history) so a UI can settle an
/// `Unconfirmed` before offering a retry. A signature that is only
/// `processed` is still in flight and is reported as
/// `Err(ActionError::Unconfirmed(signature))`.
pub fn confirm_signature(
    rpc: &RpcClient,
    signature: &Signature,
) -> Result<SignatureOutcome, ActionError> {
    let response = rpc
        .get_signature_statuses_with_history(&[*signature])
        .map_err(map_client_error)?;
    let status = response.value.into_iter().next().flatten();
    if status.is_none() {
        return Ok(SignatureOutcome::NotFound);
    }
    match classify_status(status) {
        Poll::Done(Ok(())) => Ok(SignatureOutcome::Confirmed),
        Poll::Done(Err(ActionError::Rejected(msg))) => Ok(SignatureOutcome::Failed(msg)),
        Poll::Done(Err(other)) => Err(other),
        Poll::Pending => Err(ActionError::Unconfirmed(*signature)),
    }
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
    let init_approvers =
        build_initialize_approvers_instruction(program_id, &owner.pubkey(), &vault);
    // One transaction, so a pool vault either exists with its approvers list
    // or not at all. Two transactions could leave a vault without approvers
    // that a retry can never repair (`initialize_vault` fails once the PDA exists).
    send_instructions(rpc, owner, &[init_vault, init_approvers], &[])?;

    Ok(vault)
}

/// Deposits `amount_base_units` (the token's smallest unit, so 2.5 USDC is
/// 2_500_000) from `depositor` into the vault. The UI gets `decimals` from
/// `VaultView` and converts typed text with `parse_decimal_amount`. Zero is
/// `Rejected` before any RPC.
pub fn deposit(
    rpc: &RpcClient,
    program_id: &Pubkey,
    depositor: &Keypair,
    vault: &Pubkey,
    amount_base_units: u64,
) -> Result<Signature, ActionError> {
    require_nonzero(amount_base_units)?;
    let (vault_token_account, mint) = fetch_vault_token_account_and_mint(rpc, vault)?;

    let ix = build_deposit_instruction(
        program_id,
        &depositor.pubkey(),
        vault,
        &vault_token_account,
        &mint,
        amount_base_units,
    );
    send_instructions(rpc, depositor, &[ix], &[])
}

/// Sends `amount_base_units` (the token's smallest unit) from the vault to
/// `recipient`. The recipient must already be an approved payee — the
/// program enforces this regardless of what this function sends. The UI
/// gets `decimals` from `VaultView` and converts typed text with
/// `parse_decimal_amount`. Zero is `Rejected` before any RPC.
pub fn send(
    rpc: &RpcClient,
    program_id: &Pubkey,
    owner: &Keypair,
    vault: &Pubkey,
    recipient: &Pubkey,
    amount_base_units: u64,
) -> Result<Signature, ActionError> {
    require_nonzero(amount_base_units)?;
    let (vault_token_account, mint) = fetch_vault_token_account_and_mint(rpc, vault)?;

    let ix = build_withdraw_instruction(
        program_id,
        &owner.pubkey(),
        vault,
        &vault_token_account,
        &mint,
        recipient,
        amount_base_units,
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
) -> Result<Signature, ActionError> {
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
) -> Result<Signature, ActionError> {
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
) -> Result<Signature, ActionError> {
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
) -> Result<Signature, ActionError> {
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

/// Requests an over-limit withdrawal of `amount_base_units` (the token's
/// smallest unit; the UI converts with `parse_decimal_amount` using the
/// `decimals` from `VaultView`) from a pool vault, to be later confirmed
/// with `approve_withdrawal`. Zero is `Rejected` before any RPC.
///
/// The request nonce is chosen here: the smallest unused one, from
/// `vault_view::next_free_nonce` (which errors when the scan window is
/// full). Returns the confirmed signature and the nonce used; the UI can
/// later find the request in `VaultView::pending` under that nonce. If two
/// requests race for the same nonce the program rejects the second one. Not part of the MVP's default
/// "send" flow (that goes straight through `send`/`Withdraw` for
/// within-limit amounts) but needed once a pool vault's daily limit is
/// exceeded.
pub fn request_withdrawal(
    rpc: &RpcClient,
    program_id: &Pubkey,
    owner: &Keypair,
    vault: &Pubkey,
    recipient: &Pubkey,
    amount_base_units: u64,
) -> Result<(Signature, u64), ActionError> {
    require_nonzero(amount_base_units)?;
    let (_vault_token_account, mint) = fetch_vault_token_account_and_mint(rpc, vault)?;
    let nonce = next_free_nonce(rpc, program_id, vault)?;

    let ix = build_request_withdrawal_instruction(
        program_id,
        &owner.pubkey(),
        vault,
        &mint,
        recipient,
        amount_base_units,
        nonce,
    );
    let signature = send_instructions(rpc, owner, &[ix], &[])?;
    Ok((signature, nonce))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(
        err: Option<solana_transaction::TransactionError>,
        confirmation_status: Option<TransactionConfirmationStatus>,
        confirmations: Option<usize>,
    ) -> TransactionStatus {
        TransactionStatus {
            slot: 1,
            confirmations,
            status: match &err {
                Some(e) => Err(e.clone()),
                None => Ok(()),
            },
            err,
            confirmation_status,
        }
    }

    #[test]
    fn no_status_yet_is_pending() {
        assert!(matches!(classify_status(None), Poll::Pending));
    }

    #[test]
    fn processed_only_is_still_pending() {
        let s = status(
            None,
            Some(TransactionConfirmationStatus::Processed),
            Some(0),
        );
        assert!(matches!(classify_status(Some(s)), Poll::Pending));
    }

    #[test]
    fn confirmed_without_error_is_done_ok() {
        let s = status(
            None,
            Some(TransactionConfirmationStatus::Confirmed),
            Some(5),
        );
        assert!(matches!(classify_status(Some(s)), Poll::Done(Ok(()))));
    }

    #[test]
    fn finalized_without_error_is_done_ok() {
        let s = status(None, Some(TransactionConfirmationStatus::Finalized), None);
        assert!(matches!(classify_status(Some(s)), Poll::Done(Ok(()))));
    }

    #[test]
    fn a_status_with_an_error_is_rejected_with_the_error_text() {
        let tx_err = solana_transaction::TransactionError::AccountNotFound;
        let s = status(
            Some(tx_err.clone()),
            Some(TransactionConfirmationStatus::Confirmed),
            Some(3),
        );
        match classify_status(Some(s)) {
            Poll::Done(Err(ActionError::Rejected(msg))) => assert_eq!(msg, tx_err.to_string()),
            _ => panic!("expected Done(Err(Rejected))"),
        }
    }

    #[test]
    fn a_status_from_an_old_node_without_confirmation_status_uses_confirmations() {
        let rooted = status(None, None, None);
        assert!(matches!(classify_status(Some(rooted)), Poll::Done(Ok(()))));
        let fresh = status(None, None, Some(0));
        assert!(matches!(classify_status(Some(fresh)), Poll::Pending));
    }

    /// Nothing listens here; any RPC attempt would give NoConnection, not
    /// the zero-amount rejection, so these prove the check runs first.
    fn dead_rpc() -> RpcClient {
        RpcClient::new("http://127.0.0.1:1".to_string())
    }

    fn assert_zero_rejected<T: std::fmt::Debug>(result: Result<T, ActionError>) {
        match result {
            Err(ActionError::Rejected(msg)) => {
                assert_eq!(msg, "amount must be greater than zero")
            }
            other => panic!("expected zero-amount rejection, got {other:?}"),
        }
    }

    #[test]
    fn zero_amounts_are_rejected_before_any_rpc() {
        let rpc = dead_rpc();
        let program_id = Pubkey::new_unique();
        let who = Keypair::new();
        let vault = Pubkey::new_unique();
        let other = Pubkey::new_unique();
        assert_zero_rejected(deposit(&rpc, &program_id, &who, &vault, 0));
        assert_zero_rejected(send(&rpc, &program_id, &who, &vault, &other, 0));
        assert_zero_rejected(request_withdrawal(
            &rpc,
            &program_id,
            &who,
            &vault,
            &other,
            0,
        ));
    }

    #[test]
    fn a_connection_failure_while_sending_is_unconfirmed_not_a_safe_retry() {
        let signature = Signature::from([9u8; 64]);
        let io = solana_client::client_error::ClientError::from(std::io::Error::other("reset"));
        assert!(matches!(
            send_error(io, signature),
            ActionError::Unconfirmed(s) if s == signature
        ));
    }

    #[test]
    fn a_node_rejection_while_sending_stays_rejected() {
        let signature = Signature::from([9u8; 64]);
        let custom = solana_client::client_error::ClientError::from(
            solana_client::client_error::ClientErrorKind::Custom("preflight failed".into()),
        );
        assert!(matches!(
            send_error(custom, signature),
            ActionError::Rejected(_)
        ));
    }
}
