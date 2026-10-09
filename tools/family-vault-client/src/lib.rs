//! Shared chain-facing primitives for every `family-vault` client
//! (kiosk-gui, admin-cli, and future family-app): pubkey conversion
//! between the two incompatible Solana crate generations coexisting in
//! this workspace, PDA derivation, and mint-decimals lookup. Each of
//! these was independently duplicated across `kiosk-gui/src/chain.rs`,
//! `kiosk-gui/src/setup.rs`, and `tools/admin-cli/src/main.rs` before
//! this crate existed — this is the single source of truth now.

pub mod builders;
pub mod pairing;

use solana_client::rpc_client::RpcClient;
use solana_pubkey::Pubkey;
use spl_token::solana_program::program_pack::Pack;

pub type AnchorPubkey = anchor_lang::prelude::Pubkey;

/// Converts from the client-era `solana_pubkey::Pubkey` (used by
/// `solana-client`/`solana-signer`) to the `anchor_lang::prelude::Pubkey`
/// that Anchor-generated `accounts`/`instruction` bindings require. These
/// are two distinct, incompatible types from different generations of the
/// Solana crate ecosystem that happen to coexist in this workspace.
pub fn to_anchor(p: Pubkey) -> AnchorPubkey {
    AnchorPubkey::new_from_array(p.to_bytes())
}

/// The inverse of [`to_anchor`].
pub fn to_client(p: AnchorPubkey) -> Pubkey {
    Pubkey::new_from_array(p.to_bytes())
}

/// Converts Anchor-generated `AccountMeta`s (old Solana crate generation)
/// into the client-generation `AccountMeta`s that `solana_instruction::
/// Instruction` needs. Every place that builds an instruction from an
/// Anchor `accounts` struct needs exactly this conversion.
pub fn to_client_metas(
    metas: &[anchor_lang::prelude::AccountMeta],
) -> Vec<solana_instruction::AccountMeta> {
    metas
        .iter()
        .map(|m| solana_instruction::AccountMeta {
            pubkey: to_client(m.pubkey),
            is_signer: m.is_signer,
            is_writable: m.is_writable,
        })
        .collect()
}

fn find_pda(seeds: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(seeds, program_id)
}

/// Derives a vault PDA. `is_pool` selects between the `"pool"` and
/// `"personal"` seed variants — the same owner has one of each, at
/// different addresses.
pub fn vault_pda(program_id: &Pubkey, owner: &Pubkey, is_pool: bool) -> (Pubkey, u8) {
    let kind: &[u8] = if is_pool { b"pool" } else { b"personal" };
    find_pda(&[b"vault", owner.as_ref(), kind], program_id)
}

pub fn beneficiary_list_pda(program_id: &Pubkey, vault: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"beneficiaries", vault.as_ref()], program_id)
}

pub fn approvers_pda(program_id: &Pubkey, vault: &Pubkey) -> (Pubkey, u8) {
    find_pda(&[b"approvers", vault.as_ref()], program_id)
}

pub fn request_pda(program_id: &Pubkey, vault: &Pubkey, nonce: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"request", vault.as_ref(), &nonce.to_le_bytes()],
        program_id,
    )
}

pub fn rotation_pda(program_id: &Pubkey, vault: &Pubkey, nonce: u64) -> (Pubkey, u8) {
    find_pda(
        &[b"rotation", vault.as_ref(), &nonce.to_le_bytes()],
        program_id,
    )
}

/// Decodes an SPL Mint account's `decimals` field from its raw account
/// data. Split out from [`fetch_mint_decimals`] so the decode logic is
/// testable without an RPC connection.
pub fn decode_mint_decimals(mint_account_data: &[u8]) -> Result<u32, String> {
    spl_token::state::Mint::unpack(mint_account_data)
        .map(|mint| mint.decimals as u32)
        .map_err(|e| format!("could not decode mint account: {e}"))
}

/// Converts a whole-token amount into the token's base units (`amount *
/// 10^decimals`). Checked: a silent wrap-around would move a different
/// amount of money than the caller asked for.
pub fn whole_units_to_base_units(amount_whole_units: u64, decimals: u32) -> Result<u64, String> {
    10u64
        .checked_pow(decimals)
        .and_then(|scale| amount_whole_units.checked_mul(scale))
        .ok_or_else(|| "amount is too large".to_string())
}

/// Reads decimals live from the Mint account — different SPL tokens
/// (real USDC, wrapped SOL, a custom demo token) declare different
/// decimals, so this can't be a hardcoded constant.
pub fn fetch_mint_decimals(rpc: &RpcClient, mint: &Pubkey) -> Result<u32, String> {
    let data = rpc
        .get_account_data(mint)
        .map_err(|e| format!("could not fetch mint {mint}: {e}"))?;
    decode_mint_decimals(&data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pubkey(byte: u8) -> Pubkey {
        Pubkey::new_from_array([byte; 32])
    }

    #[test]
    fn to_anchor_and_back_round_trips() {
        let original = pubkey(7);
        assert_eq!(to_client(to_anchor(original)), original);
    }

    #[test]
    fn vault_pda_is_deterministic() {
        let program_id = pubkey(1);
        let owner = pubkey(2);
        let (a, bump_a) = vault_pda(&program_id, &owner, false);
        let (b, bump_b) = vault_pda(&program_id, &owner, false);
        assert_eq!(a, b);
        assert_eq!(bump_a, bump_b);
    }

    #[test]
    fn vault_pda_differs_between_pool_and_personal_for_same_owner() {
        let program_id = pubkey(1);
        let owner = pubkey(2);
        let (personal, _) = vault_pda(&program_id, &owner, false);
        let (pool, _) = vault_pda(&program_id, &owner, true);
        assert_ne!(personal, pool);
    }

    #[test]
    fn vault_pda_differs_between_owners() {
        let program_id = pubkey(1);
        let (a, _) = vault_pda(&program_id, &pubkey(2), false);
        let (b, _) = vault_pda(&program_id, &pubkey(3), false);
        assert_ne!(a, b);
    }

    #[test]
    fn beneficiary_list_and_approvers_pdas_differ_for_the_same_vault() {
        let program_id = pubkey(1);
        let vault = pubkey(9);
        let (beneficiaries, _) = beneficiary_list_pda(&program_id, &vault);
        let (approvers, _) = approvers_pda(&program_id, &vault);
        assert_ne!(beneficiaries, approvers);
    }

    #[test]
    fn request_pda_differs_by_nonce() {
        let program_id = pubkey(1);
        let vault = pubkey(9);
        let (a, _) = request_pda(&program_id, &vault, 0);
        let (b, _) = request_pda(&program_id, &vault, 1);
        assert_ne!(a, b);
    }

    #[test]
    fn rotation_pda_differs_from_request_pda_for_the_same_vault_and_nonce() {
        let program_id = pubkey(1);
        let vault = pubkey(9);
        let (request, _) = request_pda(&program_id, &vault, 0);
        let (rotation, _) = rotation_pda(&program_id, &vault, 0);
        assert_ne!(request, rotation);
    }

    #[test]
    fn decode_mint_decimals_reads_the_real_decimals_field() {
        use spl_token::state::Mint;
        let mint = Mint {
            mint_authority: spl_token::solana_program::program_option::COption::None,
            supply: 0,
            decimals: 6,
            is_initialized: true,
            freeze_authority: spl_token::solana_program::program_option::COption::None,
        };
        let mut data = vec![0u8; Mint::LEN];
        Mint::pack(mint, &mut data).unwrap();

        assert_eq!(decode_mint_decimals(&data), Ok(6));
    }

    #[test]
    fn decode_mint_decimals_rejects_garbage() {
        let data = vec![0u8; 3];
        assert!(decode_mint_decimals(&data).is_err());
    }

    #[test]
    fn whole_units_convert_using_the_given_decimals() {
        assert_eq!(whole_units_to_base_units(5, 6), Ok(5_000_000));
        assert_eq!(whole_units_to_base_units(0, 6), Ok(0));
        assert_eq!(whole_units_to_base_units(7, 0), Ok(7));
        assert_eq!(whole_units_to_base_units(1, 9), Ok(1_000_000_000));
    }

    #[test]
    fn whole_units_refuse_to_overflow_instead_of_wrapping() {
        assert!(whole_units_to_base_units(u64::MAX, 6).is_err());
        // 10^20 does not fit in a u64 at all.
        assert!(whole_units_to_base_units(1, 20).is_err());
        // The largest amount that still fits is accepted.
        assert_eq!(
            whole_units_to_base_units(u64::MAX / 1_000_000, 6),
            Ok(u64::MAX / 1_000_000 * 1_000_000)
        );
    }

    #[test]
    fn to_client_metas_preserves_pubkey_and_flags() {
        let key = pubkey(5);
        let metas = vec![
            anchor_lang::prelude::AccountMeta::new(to_anchor(key), true),
            anchor_lang::prelude::AccountMeta::new_readonly(to_anchor(pubkey(6)), false),
        ];
        let converted = to_client_metas(&metas);
        assert_eq!(converted.len(), 2);
        assert_eq!(converted[0].pubkey, key);
        assert!(converted[0].is_signer && converted[0].is_writable);
        assert_eq!(converted[1].pubkey, pubkey(6));
        assert!(!converted[1].is_signer && !converted[1].is_writable);
    }
}
