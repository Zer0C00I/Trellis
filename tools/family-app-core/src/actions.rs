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
use solana_client::rpc_config::CommitmentConfig;
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
/// `Unconfirmed`. This is only how long the call blocks; it says nothing
/// about when a retry becomes safe (that is decided by block height, see
/// [`confirm_signature`]).
const CONFIRM_DEADLINE: Duration = Duration::from_secs(60);
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// How many consecutive failed status lookups `send_instructions` rides out
/// before giving up. The 4th failure in a row ends the wait.
const MAX_CONSECUTIVE_POLL_ERRORS: u32 = 3;

/// What one look at a signature's status means.
#[derive(Debug, PartialEq, Eq)]
enum Poll {
    /// Not seen yet, or only `processed` (could still be dropped).
    Pending,
    /// Confirmed or finalized without error.
    Confirmed,
    /// Executed and failed (the transaction error text): it had no effect.
    Failed(String),
}

fn classify_status(status: Option<TransactionStatus>) -> Poll {
    let Some(status) = status else {
        return Poll::Pending;
    };
    if let Some(err) = status.err {
        // The transaction executed and failed: it definitely did not take effect.
        return Poll::Failed(err.to_string());
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
        Poll::Confirmed
    } else {
        Poll::Pending
    }
}

/// Counts consecutive failed status lookups. Returns the new count and
/// whether the caller must give up (more than [`MAX_CONSECUTIVE_POLL_ERRORS`]
/// failures in a row). A successful lookup resets the count.
fn track_poll_result(consecutive_errors: u32, lookup_succeeded: bool) -> (u32, bool) {
    if lookup_succeeded {
        return (0, false);
    }
    let count = consecutive_errors.saturating_add(1);
    (count, count > MAX_CONSECUTIVE_POLL_ERRORS)
}

/// Maps an error from `send_transaction`. Once the request may have reached
/// the node, only a DEFINITE refusal is `Rejected`: an RPC error response
/// (preflight failure, unhealthy node, ... - in solana-rpc-client 4.2.2 a
/// preflight failure arrives as `RpcError::RpcResponseError` with
/// `SendTransactionPreflightFailure` data, see `http_sender.rs`) or a
/// `TransactionError`. Everything else (I/O, HTTP transport, unparsable or
/// mismatched responses, JSON errors, middleware, custom) can happen AFTER
/// the node accepted the transaction, so the outcome is `Unconfirmed`.
fn send_error(
    err: solana_client::client_error::ClientError,
    signature: Signature,
    last_valid_block_height: u64,
) -> ActionError {
    use solana_client::client_error::ClientErrorKind;
    use solana_client::rpc_request::RpcError;
    match err.kind() {
        ClientErrorKind::RpcError(RpcError::RpcResponseError { .. })
        | ClientErrorKind::TransactionError(_) => map_client_error(err),
        _ => ActionError::Unconfirmed {
            signature,
            last_valid_block_height,
        },
    }
}

/// Sends one or more instructions in a single transaction, signed by
/// `payer` plus any `extra_signers` (e.g. a second trusted approver for
/// `add_beneficiary_urgent`, or an existing approver co-signing a new
/// one).
///
/// The transaction is signed once, sent, and then its signature is polled
/// for up to ~60s (this only bounds how long the call blocks), or until its
/// blockhash has expired. A few consecutive failed status lookups are
/// tolerated. Retry rule for callers:
///
/// - `Ok(signature)`: confirmed on chain. Never resend.
/// - `Rejected` / `NoConnection`: the transaction was NOT accepted (the node
///   definitely refused it, it failed on chain, or no connection could be
///   made before anything was sent), so building a fresh transaction and
///   retrying is safe.
/// - `Unconfirmed { signature, last_valid_block_height }`: it was sent (or
///   the connection broke while sending) and the outcome is unknown. Do NOT
///   retry with a new transaction (new blockhash, would pay twice). Call
///   [`confirm_signature`] with both values, as often as needed. A retry is
///   safe only after it returns `Failed` (landed with an error) or `Expired`
///   (not found and the chain is past `last_valid_block_height`, so it can
///   never land). `StillPossible` means: do not retry, check again later.
///   For `request_withdrawal` the retry must additionally reuse the same nonce.
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
    let unconfirmed = ActionError::Unconfirmed {
        signature,
        last_valid_block_height,
    };
    let signature = match rpc.send_transaction(&tx) {
        Ok(returned) => returned,
        Err(e) => return Err(send_error(e, signature, last_valid_block_height)),
    };

    let deadline = Instant::now() + CONFIRM_DEADLINE;
    let mut consecutive_errors = 0u32;
    loop {
        let lookup = rpc.get_signature_statuses(&[signature]);
        let (count, give_up) = track_poll_result(consecutive_errors, lookup.is_ok());
        consecutive_errors = count;
        if give_up {
            return Err(unconfirmed);
        }
        if let Ok(response) = lookup {
            match classify_status(response.value.into_iter().next().flatten()) {
                Poll::Confirmed => return Ok(signature),
                Poll::Failed(msg) => return Err(ActionError::Rejected(msg)),
                Poll::Pending => {}
            }
        }
        if Instant::now() >= deadline {
            return Err(unconfirmed);
        }
        // Stop early once the blockhash has expired; a failed height lookup
        // just means "keep polling until the deadline".
        if current_block_height(rpc).is_ok_and(|height| height > last_valid_block_height) {
            return Err(unconfirmed);
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// The node's block height at the `confirmed` commitment level, regardless
/// of the client's own commitment. `confirmed` can only trail the leader's
/// tip, never run ahead of it, so "confirmed height is past the last valid
/// height" really means the blockhash can no longer be accepted; it also
/// does not lag as far behind as `finalized` (the default client level).
fn current_block_height(rpc: &RpcClient) -> Result<u64, solana_client::client_error::ClientError> {
    rpc.get_block_height_with_commitment(CommitmentConfig::confirmed())
}

/// What the network knows about a signature, for resolving an
/// `ActionError::Unconfirmed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureOutcome {
    /// Landed and confirmed or finalized without error.
    Confirmed,
    /// Landed and failed (the transaction error text); it had no effect.
    /// Safe to retry.
    Failed(String),
    /// Not found, and the chain is past the transaction's last valid block
    /// height: it can never land. Safe to retry.
    Expired,
    /// Not found, or only `processed`, and the blockhash has not expired: it
    /// may still land. Do NOT retry; check again later.
    StillPossible,
}

/// The pure decision behind [`confirm_signature`].
///
/// `last_valid_block_height` is inclusive (a transaction can still land at
/// exactly that height), so `Expired` needs `current_block_height` strictly
/// greater. A status that exists but is not yet `confirmed` is never
/// `Expired`: the transaction was seen, so it may still be finalized.
fn settle(
    status: Option<TransactionStatus>,
    current_block_height: u64,
    last_valid_block_height: u64,
) -> SignatureOutcome {
    match (status.is_some(), classify_status(status)) {
        (_, Poll::Confirmed) => SignatureOutcome::Confirmed,
        (_, Poll::Failed(msg)) => SignatureOutcome::Failed(msg),
        (true, Poll::Pending) => SignatureOutcome::StillPossible,
        (false, Poll::Pending) => {
            if current_block_height > last_valid_block_height {
                SignatureOutcome::Expired
            } else {
                SignatureOutcome::StillPossible
            }
        }
    }
}

/// Resolves an `ActionError::Unconfirmed` by asking the node about the
/// signature (`get_signature_statuses_with_history`, i.e. with
/// `searchTransactionHistory`) and its block height (at `confirmed`
/// commitment). See [`SignatureOutcome`] and [`settle`] for the rules.
///
/// The block height is read BEFORE the status on purpose: if the height was
/// already past `last_valid_block_height` and the status read afterwards
/// still finds nothing, the transaction cannot have landed in between. The
/// reverse order could miss a transaction that lands between the two reads.
pub fn confirm_signature(
    rpc: &RpcClient,
    signature: &Signature,
    last_valid_block_height: u64,
) -> Result<SignatureOutcome, ActionError> {
    let height = current_block_height(rpc).map_err(map_client_error)?;
    let response = rpc
        .get_signature_statuses_with_history(&[*signature])
        .map_err(map_client_error)?;
    let status = response.value.into_iter().next().flatten();
    Ok(settle(status, height, last_valid_block_height))
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
/// The caller picks `nonce`: take it from `VaultView::next_free_nonce` (or
/// `vault_view::next_free_nonce`), and STORE it before calling. On
/// `ActionError::Unconfirmed`, resolve with [`confirm_signature`] and, when
/// a retry is safe, retry with the SAME nonce, never a fresh one. The
/// request account is created with `init` at a PDA derived from the nonce,
/// so a second attempt with the same nonce can succeed only once (a
/// duplicate fails with "account already in use"). A new nonce would create
/// a second, identical pending request, and two approvals could then each
/// move the amount. If another device took the nonce first, the program
/// rejects this request and no funds move. Not part of the MVP's default
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
    nonce: u64,
) -> Result<Signature, ActionError> {
    require_nonzero(amount_base_units)?;
    let (_vault_token_account, mint) = fetch_vault_token_account_and_mint(rpc, vault)?;

    let ix = build_request_withdrawal_instruction(
        program_id,
        &owner.pubkey(),
        vault,
        &mint,
        recipient,
        amount_base_units,
        nonce,
    );
    send_instructions(rpc, owner, &[ix], &[])
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_client::client_error::{ClientError, ClientErrorKind};
    use solana_client::rpc_request::{RpcError, RpcResponseErrorData};

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
        assert_eq!(classify_status(None), Poll::Pending);
    }

    #[test]
    fn processed_only_is_still_pending() {
        let s = status(
            None,
            Some(TransactionConfirmationStatus::Processed),
            Some(0),
        );
        assert_eq!(classify_status(Some(s)), Poll::Pending);
    }

    #[test]
    fn confirmed_without_error_is_done_ok() {
        let s = status(
            None,
            Some(TransactionConfirmationStatus::Confirmed),
            Some(5),
        );
        assert_eq!(classify_status(Some(s)), Poll::Confirmed);
    }

    #[test]
    fn finalized_without_error_is_done_ok() {
        let s = status(None, Some(TransactionConfirmationStatus::Finalized), None);
        assert_eq!(classify_status(Some(s)), Poll::Confirmed);
    }

    #[test]
    fn a_status_with_an_error_is_rejected_with_the_error_text() {
        let tx_err = solana_transaction::TransactionError::AccountNotFound;
        let s = status(
            Some(tx_err.clone()),
            Some(TransactionConfirmationStatus::Confirmed),
            Some(3),
        );
        assert_eq!(classify_status(Some(s)), Poll::Failed(tx_err.to_string()));
    }

    #[test]
    fn a_status_from_an_old_node_without_confirmation_status_uses_confirmations() {
        let rooted = status(None, None, None);
        assert_eq!(classify_status(Some(rooted)), Poll::Confirmed);
        let fresh = status(None, None, Some(0));
        assert_eq!(classify_status(Some(fresh)), Poll::Pending);
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
            7,
        ));
    }

    const SIG: [u8; 64] = [9u8; 64];
    const LAST_VALID: u64 = 1_000;

    fn client_err(kind: solana_client::client_error::ClientErrorKind) -> ClientError {
        ClientError::from(kind)
    }

    fn assert_unconfirmed(err: ActionError) {
        match err {
            ActionError::Unconfirmed {
                signature,
                last_valid_block_height,
            } => {
                assert_eq!(signature, Signature::from(SIG));
                assert_eq!(last_valid_block_height, LAST_VALID);
            }
            other => panic!("expected Unconfirmed, got {other:?}"),
        }
    }

    fn sent(err: ClientError) -> ActionError {
        send_error(err, Signature::from(SIG), LAST_VALID)
    }

    #[test]
    fn io_error_while_sending_is_unconfirmed() {
        assert_unconfirmed(sent(ClientError::from(std::io::Error::other("reset"))));
    }

    #[test]
    fn parse_error_while_sending_is_unconfirmed() {
        assert_unconfirmed(sent(client_err(ClientErrorKind::RpcError(
            RpcError::ParseError("Signature".into()),
        ))));
    }

    #[test]
    fn mismatched_signature_request_error_is_unconfirmed() {
        assert_unconfirmed(sent(client_err(ClientErrorKind::RpcError(
            RpcError::RpcRequestError("RPC node returned mismatched signature".into()),
        ))));
    }

    #[test]
    fn for_user_error_while_sending_is_unconfirmed() {
        assert_unconfirmed(sent(client_err(ClientErrorKind::RpcError(
            RpcError::ForUser("odd".into()),
        ))));
    }

    #[test]
    fn serde_json_error_while_sending_is_unconfirmed() {
        let json_err = serde_json::from_str::<u8>("not json").unwrap_err();
        assert_unconfirmed(sent(client_err(ClientErrorKind::SerdeJson(json_err))));
    }

    #[test]
    fn custom_error_while_sending_is_unconfirmed() {
        assert_unconfirmed(sent(client_err(ClientErrorKind::Custom("x".into()))));
    }

    #[test]
    fn an_rpc_error_response_while_sending_is_rejected() {
        let refused = client_err(ClientErrorKind::RpcError(RpcError::RpcResponseError {
            code: -32002,
            message: "Transaction simulation failed".into(),
            data: RpcResponseErrorData::Empty,
        }));
        match sent(refused) {
            ActionError::Rejected(msg) => assert!(msg.contains("simulation failed"), "{msg}"),
            other => panic!("expected Rejected, got {other:?}"),
        }
    }

    #[test]
    fn a_transaction_error_while_sending_is_rejected() {
        let refused = client_err(ClientErrorKind::TransactionError(
            solana_transaction::TransactionError::AccountNotFound,
        ));
        assert!(matches!(sent(refused), ActionError::Rejected(_)));
    }

    #[test]
    fn up_to_three_consecutive_poll_errors_are_tolerated_the_fourth_is_not() {
        let mut count = 0;
        for _ in 0..3 {
            let (c, give_up) = track_poll_result(count, false);
            assert!(!give_up);
            count = c;
        }
        assert_eq!(count, 3);
        let (count, give_up) = track_poll_result(count, false);
        assert_eq!(count, 4);
        assert!(give_up);
    }

    #[test]
    fn a_successful_poll_resets_the_error_count() {
        assert_eq!(track_poll_result(3, true), (0, false));
        let (c, give_up) = track_poll_result(0, false);
        assert_eq!((c, give_up), (1, false));
        assert_eq!(track_poll_result(u32::MAX, false).0, u32::MAX); // saturates
    }

    #[test]
    fn settle_confirmed_and_finalized() {
        let c = status(
            None,
            Some(TransactionConfirmationStatus::Confirmed),
            Some(2),
        );
        assert_eq!(
            settle(Some(c), LAST_VALID + 500, LAST_VALID),
            SignatureOutcome::Confirmed
        );
        let f = status(None, Some(TransactionConfirmationStatus::Finalized), None);
        assert_eq!(settle(Some(f), 0, LAST_VALID), SignatureOutcome::Confirmed);
    }

    #[test]
    fn settle_processed_only_is_still_possible_even_past_expiry() {
        let p = status(
            None,
            Some(TransactionConfirmationStatus::Processed),
            Some(0),
        );
        assert_eq!(
            settle(Some(p.clone()), 0, LAST_VALID),
            SignatureOutcome::StillPossible
        );
        assert_eq!(
            settle(Some(p), LAST_VALID + 500, LAST_VALID),
            SignatureOutcome::StillPossible
        );
    }

    #[test]
    fn settle_error_is_failed() {
        let tx_err = solana_transaction::TransactionError::AccountNotFound;
        let s = status(
            Some(tx_err.clone()),
            Some(TransactionConfirmationStatus::Finalized),
            None,
        );
        assert_eq!(
            settle(Some(s), 0, LAST_VALID),
            SignatureOutcome::Failed(tx_err.to_string())
        );
    }

    #[test]
    fn settle_not_found_before_expiry_is_still_possible() {
        assert_eq!(
            settle(None, LAST_VALID - 1, LAST_VALID),
            SignatureOutcome::StillPossible
        );
    }

    #[test]
    fn settle_not_found_at_the_last_valid_height_is_still_possible() {
        assert_eq!(
            settle(None, LAST_VALID, LAST_VALID),
            SignatureOutcome::StillPossible
        );
    }

    #[test]
    fn settle_not_found_after_expiry_is_expired() {
        assert_eq!(
            settle(None, LAST_VALID + 1, LAST_VALID),
            SignatureOutcome::Expired
        );
    }
}
