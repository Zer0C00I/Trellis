//! Everyday bank-style actions, each a thin RPC-send/sign wrapper around
//! one of `family_vault_client::builders`' pure instruction builders.
//! Covers the same everyday commands as `admin-cli` (see that tool's
//! `cmd_*` functions), minus owner rotation, but returns `Result` instead of printing and calling
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
use family_vault_client::{decode_mint_decimals, to_client, vault_pda};
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

fn whole_units_to_base_units(amount_whole_units: u64, decimals: u32) -> Result<u64, ActionError> {
    family_vault_client::whole_units_to_base_units(amount_whole_units, decimals)
        .map_err(ActionError::Rejected)
}

fn mint_decimals(rpc: &RpcClient, mint: &Pubkey) -> Result<u32, ActionError> {
    let data = rpc.get_account_data(mint).map_err(map_client_error)?;
    decode_mint_decimals(&data).map_err(ActionError::Rejected)
}

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
///
/// Caution for callers: the transaction is sent first and then polled for
/// confirmation, so `NoConnection` (or a `Rejected` expiry) can mean the
/// outcome is *unknown*, not that nothing happened. Check the account state
/// before offering a retry, or a payment may go out twice.
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
    rpc.send_and_confirm_transaction(&tx)
        .map_err(map_client_error)
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

pub fn deposit(
    rpc: &RpcClient,
    program_id: &Pubkey,
    depositor: &Keypair,
    vault: &Pubkey,
    amount_whole_units: u64,
) -> Result<solana_signature::Signature, ActionError> {
    let (vault_token_account, mint) = fetch_vault_token_account_and_mint(rpc, vault)?;
    let decimals = mint_decimals(rpc, &mint)?;
    let amount = whole_units_to_base_units(amount_whole_units, decimals)?;

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
    let decimals = mint_decimals(rpc, &mint)?;
    let amount = whole_units_to_base_units(amount_whole_units, decimals)?;

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
pub fn request_withdrawal(
    rpc: &RpcClient,
    program_id: &Pubkey,
    owner: &Keypair,
    vault: &Pubkey,
    recipient: &Pubkey,
    amount_whole_units: u64,
    nonce: u64,
) -> Result<solana_signature::Signature, ActionError> {
    let (_vault_token_account, mint) = fetch_vault_token_account_and_mint(rpc, vault)?;
    let decimals = mint_decimals(rpc, &mint)?;
    let amount = whole_units_to_base_units(amount_whole_units, decimals)?;

    let ix = build_request_withdrawal_instruction(
        program_id,
        &owner.pubkey(),
        vault,
        &mint,
        recipient,
        amount,
        nonce,
    );
    send_instructions(rpc, owner, &[ix], &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_io_errors_to_no_connection() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        let client_err: solana_client::client_error::ClientError = io_err.into();
        assert!(matches!(
            map_client_error(client_err),
            ActionError::NoConnection
        ));
    }

    #[test]
    fn maps_other_errors_to_rejected() {
        let client_err: solana_client::client_error::ClientError =
            solana_client::client_error::ClientErrorKind::Custom("nope".to_string()).into();
        assert!(matches!(
            map_client_error(client_err),
            ActionError::Rejected(_)
        ));
    }

    #[test]
    fn converts_whole_units_to_base_units() {
        assert_eq!(whole_units_to_base_units(5, 6).unwrap(), 5_000_000);
        assert_eq!(whole_units_to_base_units(0, 6).unwrap(), 0);
        assert_eq!(whole_units_to_base_units(7, 0).unwrap(), 7);
    }

    #[test]
    fn refuses_amounts_that_would_overflow_instead_of_wrapping() {
        assert!(matches!(
            whole_units_to_base_units(u64::MAX, 6),
            Err(ActionError::Rejected(_))
        ));
        // 10^20 does not fit in a u64 at all.
        assert!(matches!(
            whole_units_to_base_units(1, 20),
            Err(ActionError::Rejected(_))
        ));
    }
}
