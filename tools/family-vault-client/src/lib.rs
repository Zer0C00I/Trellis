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

/// Parses a user-typed decimal amount ("2", "2.5", "0.01") into the token's
/// base units, exactly (no floating point).
///
/// Accepted: optional surrounding whitespace, ASCII digits, and at most one
/// '.' with digits on both sides. Rejected with a plain-English message:
/// empty text, signs, inner whitespace, exponents, thousands separators,
/// ".5" / "2.", non-ASCII digits, more fractional digits than `decimals`
/// (never rounded), zero, and anything that overflows a `u64`.
pub fn parse_decimal_amount(text: &str, decimals: u32) -> Result<u64, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("enter an amount".to_string());
    }
    let (int_part, frac_part) = match text.split_once('.') {
        Some((i, f)) => (i, f),
        None => (text, ""),
    };
    let has_dot = text.contains('.');
    if int_part.is_empty() || (has_dot && frac_part.is_empty()) {
        return Err("write the amount with digits on both sides of the dot".to_string());
    }
    let all_digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if !all_digits(int_part) || !all_digits(frac_part) {
        return Err("the amount may contain only digits and one dot".to_string());
    }
    if frac_part.len() as u64 > u64::from(decimals) {
        return Err(format!(
            "too many digits after the dot (at most {decimals} allowed)"
        ));
    }
    let too_large = || "amount is too large".to_string();
    let mut value: u64 = 0;
    for b in int_part.bytes().chain(frac_part.bytes()) {
        value = value
            .checked_mul(10)
            .and_then(|v| v.checked_add(u64::from(b - b'0')))
            .ok_or_else(too_large)?;
    }
    // Pad with the missing fractional zeros. Zero stays zero, so skip the
    // loop then (decimals can be huge).
    if value != 0 {
        for _ in 0..(u64::from(decimals) - frac_part.len() as u64) {
            value = value.checked_mul(10).ok_or_else(too_large)?;
        }
    }
    if value == 0 {
        return Err("amount must be greater than zero".to_string());
    }
    Ok(value)
}

/// Formats base units as an exact decimal string: trailing fractional zeros
/// are trimmed and a trailing dot is never printed ("2500000", 6 -> "2.5").
pub fn format_base_units(amount: u64, decimals: u32) -> String {
    if decimals == 0 {
        return amount.to_string();
    }
    let decimals = decimals as usize;
    let digits = amount.to_string();
    let padded = format!("{digits:0>width$}", width = decimals + 1);
    let (int_part, frac_part) = padded.split_at(padded.len() - decimals);
    let frac_part = frac_part.trim_end_matches('0');
    if frac_part.is_empty() {
        int_part.to_string()
    } else {
        format!("{int_part}.{frac_part}")
    }
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
    fn parse_decimal_accepts_plain_and_fractional_amounts() {
        assert_eq!(parse_decimal_amount("2", 6), Ok(2_000_000));
        assert_eq!(parse_decimal_amount("2.5", 6), Ok(2_500_000));
        assert_eq!(parse_decimal_amount("2.50", 6), Ok(2_500_000));
        assert_eq!(parse_decimal_amount("0.01", 6), Ok(10_000));
        assert_eq!(parse_decimal_amount("0.000001", 6), Ok(1));
        assert_eq!(parse_decimal_amount("  3.25\n", 2), Ok(325));
        assert_eq!(parse_decimal_amount("007", 0), Ok(7));
        assert_eq!(parse_decimal_amount("1.000000000", 9), Ok(1_000_000_000));
    }

    #[test]
    fn parse_decimal_rejects_malformed_text() {
        for bad in [
            "",
            "   ",
            ".5",
            "2.",
            ".",
            "-1",
            "+1",
            "1 0",
            "1e3",
            "1E3",
            "1,000",
            "1_000",
            "1.2.3",
            "0x10",
            "abc",
            "\u{0662}",
            "2.\u{0665}",
            "１２",
            "NaN",
            "inf",
            "1..2",
        ] {
            assert!(
                parse_decimal_amount(bad, 6).is_err(),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn parse_decimal_never_rounds() {
        assert!(parse_decimal_amount("0.0000001", 6).is_err());
        assert!(parse_decimal_amount("1.5", 0).is_err());
        assert!(parse_decimal_amount("1.0", 0).is_err());
        assert_eq!(parse_decimal_amount("1.0", 1), Ok(10));
    }

    #[test]
    fn parse_decimal_rejects_zero() {
        for zero in ["0", "0.0", "0.000000", "000"] {
            assert_eq!(
                parse_decimal_amount(zero, 6),
                Err("amount must be greater than zero".to_string()),
                "{zero:?}"
            );
        }
        assert!(parse_decimal_amount("0", 4_000_000_000).is_err());
    }

    #[test]
    fn parse_decimal_checks_overflow() {
        assert_eq!(
            parse_decimal_amount("18446744073709551615", 0),
            Ok(u64::MAX)
        );
        assert!(parse_decimal_amount("18446744073709551616", 0).is_err());
        assert!(parse_decimal_amount("18446744073710", 6).is_err());
        assert_eq!(
            parse_decimal_amount("18446744073709.551615", 6),
            Ok(u64::MAX)
        );
        assert!(parse_decimal_amount("18446744073709.551616", 6).is_err());
        assert!(parse_decimal_amount("1", 20).is_err());
        assert!(parse_decimal_amount("1", u32::MAX).is_err());
        assert!(parse_decimal_amount("99999999999999999999999", 0).is_err());
    }

    #[test]
    fn format_base_units_is_exact_and_trimmed() {
        assert_eq!(format_base_units(2_500_000, 6), "2.5");
        assert_eq!(format_base_units(2_000_000, 6), "2");
        assert_eq!(format_base_units(1, 6), "0.000001");
        assert_eq!(format_base_units(0, 6), "0");
        assert_eq!(format_base_units(10_000, 6), "0.01");
        assert_eq!(format_base_units(123, 0), "123");
        assert_eq!(format_base_units(u64::MAX, 0), "18446744073709551615");
        assert_eq!(format_base_units(u64::MAX, 6), "18446744073709.551615");
        assert_eq!(format_base_units(5, 9), "0.000000005");
    }

    #[test]
    fn parse_and_format_round_trip() {
        let samples = [
            1u64,
            2,
            9,
            10,
            99,
            100,
            12_345,
            999_999,
            1_000_000,
            2_500_000,
            123_456_789,
            u64::MAX / 1_000_000,
            u64::MAX / 3,
            u64::MAX - 1,
            u64::MAX,
        ];
        for decimals in [0u32, 1, 2, 6, 9, 18, 19, 20] {
            for &x in &samples {
                let text = format_base_units(x, decimals);
                assert_eq!(
                    parse_decimal_amount(&text, decimals),
                    Ok(x),
                    "x={x} decimals={decimals} text={text}"
                );
            }
        }
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
