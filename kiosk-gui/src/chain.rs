use std::fmt;
use std::path::Path;

use serde::Deserialize;

/// SPL token decimals for the demo token used on devnet. Fixed, not read
/// from the Mint account — see the design spec's "Decimals" section.
#[allow(dead_code)] // used starting in Task 2
pub const TOKEN_DECIMALS: u32 = 6;

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // used starting in Task 5
struct RawConfig {
    rpc_url: String,
    keypair_path: String,
    program_id: String,
    mint: String,
    personal_vault: String,
    personal_beneficiary_list: String,
    pool_vault: String,
    pool_beneficiary_list: String,
}

#[derive(Debug)]
#[allow(dead_code)] // used starting in Task 5
pub struct ChainConfig {
    pub rpc_url: String,
    pub keypair_path: String,
    pub program_id: anchor_lang::prelude::Pubkey,
    pub mint: solana_pubkey::Pubkey,
    pub personal_vault: solana_pubkey::Pubkey,
    pub personal_beneficiary_list: solana_pubkey::Pubkey,
    pub pool_vault: solana_pubkey::Pubkey,
    pub pool_beneficiary_list: solana_pubkey::Pubkey,
}

#[derive(Debug)]
#[allow(dead_code)] // used starting in Task 5
pub enum ConfigError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    BadPubkey { field: &'static str, value: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "could not read config file: {e}"),
            ConfigError::Parse(e) => write!(f, "could not parse config TOML: {e}"),
            ConfigError::BadPubkey { field, value } => {
                write!(f, "config field '{field}' is not a valid pubkey: '{value}'")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[allow(dead_code)] // used starting in Task 5
fn parse_client_pubkey(field: &'static str, value: &str) -> Result<solana_pubkey::Pubkey, ConfigError> {
    value.parse().map_err(|_| ConfigError::BadPubkey {
        field,
        value: value.to_string(),
    })
}

#[allow(dead_code)] // used starting in Task 5
pub fn load_config(path: &Path) -> Result<ChainConfig, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
    let raw: RawConfig = toml::from_str(&text).map_err(ConfigError::Parse)?;

    let program_id: anchor_lang::prelude::Pubkey =
        raw.program_id.parse().map_err(|_| ConfigError::BadPubkey {
            field: "program_id",
            value: raw.program_id.clone(),
        })?;

    Ok(ChainConfig {
        rpc_url: raw.rpc_url,
        keypair_path: raw.keypair_path,
        program_id,
        mint: parse_client_pubkey("mint", &raw.mint)?,
        personal_vault: parse_client_pubkey("personal_vault", &raw.personal_vault)?,
        personal_beneficiary_list: parse_client_pubkey(
            "personal_beneficiary_list",
            &raw.personal_beneficiary_list,
        )?,
        pool_vault: parse_client_pubkey("pool_vault", &raw.pool_vault)?,
        pool_beneficiary_list: parse_client_pubkey(
            "pool_beneficiary_list",
            &raw.pool_beneficiary_list,
        )?,
    })
}

pub fn format_amount(whole_units: u64) -> String {
    format!("{whole_units} USDC")
}

#[allow(dead_code)] // used starting in Task 4
pub fn parse_amount_to_base_units(whole_units: u64) -> u64 {
    whole_units * 10u64.pow(TOKEN_DECIMALS)
}

pub fn decode_beneficiary_name(name: &[u8; 32], name_len: u8) -> String {
    let len = name_len as usize;
    String::from_utf8_lossy(&name[..len.min(32)]).into_owned()
}

#[derive(Debug, PartialEq, Eq)]
#[allow(dead_code)] // used starting in Task 5
pub struct VaultSnapshot {
    pub balance_text: String,
    pub beneficiaries: Vec<(u8, String)>,
}

#[derive(Debug)]
#[allow(dead_code)] // used starting in Task 5
pub enum ChainError {
    /// RPC endpoint unreachable — maps to "Нет связи" on Экран 4.
    NoConnection,
    /// RPC responded but the program/transaction rejected the request, or
    /// any other non-network failure — maps to "Не получилось" on Экран 4.
    ProgramRejected(String),
}

impl fmt::Display for ChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainError::NoConnection => write!(f, "no connection to RPC endpoint"),
            ChainError::ProgramRejected(msg) => write!(f, "program rejected: {msg}"),
        }
    }
}

impl std::error::Error for ChainError {}

#[allow(dead_code)] // used starting in Task 5
pub fn fetch_vault_snapshot(cfg: &ChainConfig, is_pool: bool) -> Result<VaultSnapshot, ChainError> {
    let rpc = solana_client::rpc_client::RpcClient::new(cfg.rpc_url.clone());

    let (vault_pubkey, list_pubkey) = if is_pool {
        (cfg.pool_vault, cfg.pool_beneficiary_list)
    } else {
        (cfg.personal_vault, cfg.personal_beneficiary_list)
    };

    let vault_bytes = rpc
        .get_account_data(&vault_pubkey)
        .map_err(map_client_error)?;
    let list_bytes = rpc
        .get_account_data(&list_pubkey)
        .map_err(map_client_error)?;

    // Read the vault's token account balance directly (already in whole
    // units via `ui_amount`, so no manual decimals math needed here).
    use anchor_lang::AccountDeserialize;
    let vault = family_vault::state::Vault::try_deserialize(&mut &vault_bytes[..])
        .map_err(|e| ChainError::ProgramRejected(e.to_string()))?;
    let vault_token_pubkey =
        solana_pubkey::Pubkey::new_from_array(vault.vault_token_account.to_bytes());
    let balance = rpc
        .get_token_account_balance(&vault_token_pubkey)
        .map_err(map_client_error)?;
    let whole_unit_balance: u64 = balance
        .ui_amount
        .map(|f| f.round() as u64)
        .unwrap_or(0);

    decode_snapshot(&vault_bytes, &list_bytes, whole_unit_balance)
}

fn map_client_error(err: solana_client::client_error::ClientError) -> ChainError {
    match err.kind() {
        solana_client::client_error::ClientErrorKind::Io(_)
        | solana_client::client_error::ClientErrorKind::Reqwest(_) => ChainError::NoConnection,
        other => ChainError::ProgramRejected(other.to_string()),
    }
}

/// Pure decode step, separated from `fetch_vault_snapshot` so it's testable
/// without a live RPC endpoint. `whole_unit_balance` is the vault's token
/// account balance already converted to whole units by the caller (see
/// `fetch_vault_snapshot`, which reads it via a separate RPC call).
fn decode_snapshot(
    vault_bytes: &[u8],
    beneficiary_list_bytes: &[u8],
    whole_unit_balance: u64,
) -> Result<VaultSnapshot, ChainError> {
    use anchor_lang::AccountDeserialize;
    use family_vault::state::BeneficiaryList;

    let list = BeneficiaryList::try_deserialize(&mut &beneficiary_list_bytes[..])
        .map_err(|e| ChainError::ProgramRejected(e.to_string()))?;

    let beneficiaries = (0..list.count as usize)
        .map(|i| {
            let b = &list.entries[i];
            ((i + 1) as u8, decode_beneficiary_name(&b.name, b.name_len))
        })
        .collect();

    // `vault_bytes` isn't read for balance (the token balance comes from a
    // separate token-account RPC call in `fetch_vault_snapshot`), but it's
    // still deserialized here so a corrupt/wrong-shaped Vault account fails
    // loudly instead of silently producing a snapshot from a bogus account.
    use family_vault::state::Vault;
    Vault::try_deserialize(&mut &vault_bytes[..])
        .map_err(|e| ChainError::ProgramRejected(e.to_string()))?;

    Ok(VaultSnapshot {
        balance_text: format_amount(whole_unit_balance),
        beneficiaries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp_config(contents: &str) -> std::path::PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let dir = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let path = dir.join(format!("kiosk-test-config-{}.toml", nanos));
        std::fs::write(&path, contents).unwrap();
        path
    }

    const VALID_TOML: &str = "rpc_url = \"https://api.devnet.solana.com\"\n\
keypair_path = \"/etc/family-vault/owner-keypair.json\"\n\
program_id = \"FXFxTexyhpdjmb6s2kvjVeY2aJFD9jmW8PsfSUY6LF7m\"\n\
mint = \"Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB\"\n\
personal_vault = \"11111111111111111111111111111111\"\n\
personal_beneficiary_list = \"11111111111111111111111111111111\"\n\
pool_vault = \"11111111111111111111111111111111\"\n\
pool_beneficiary_list = \"11111111111111111111111111111111\"";

    #[test]
    fn loads_a_valid_config() {
        let path = write_temp_config(VALID_TOML);
        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.rpc_url, "https://api.devnet.solana.com");
        assert_eq!(cfg.keypair_path, "/etc/family-vault/owner-keypair.json");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn rejects_missing_file() {
        let path = std::env::temp_dir().join("kiosk-test-config-does-not-exist.toml");
        let err = load_config(&path).unwrap_err();
        assert!(matches!(err, ConfigError::Io(_)));
    }

    #[test]
    fn rejects_bad_pubkey() {
        let bad = VALID_TOML.replace(
            "mint = \"Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB\"",
            "mint = \"not-a-pubkey\"",
        );
        let path = write_temp_config(&bad);
        let err = load_config(&path).unwrap_err();
        assert!(matches!(err, ConfigError::BadPubkey { field: "mint", .. }));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn formats_whole_units_as_usdc_string() {
        assert_eq!(format_amount(10), "10 USDC");
        assert_eq!(format_amount(0), "0 USDC");
        assert_eq!(format_amount(120), "120 USDC");
    }

    #[test]
    fn converts_whole_units_to_base_units_with_six_decimals() {
        assert_eq!(parse_amount_to_base_units(1), 1_000_000);
        assert_eq!(parse_amount_to_base_units(10), 10_000_000);
        assert_eq!(parse_amount_to_base_units(0), 0);
    }

    #[test]
    fn decodes_a_beneficiary_name_from_fixed_size_buffer() {
        let mut name = [0u8; 32];
        let src = "Внук Саша".as_bytes();
        name[..src.len()].copy_from_slice(src);
        assert_eq!(
            decode_beneficiary_name(&name, src.len() as u8),
            "Внук Саша"
        );
    }

    #[test]
    fn decodes_an_empty_beneficiary_name() {
        let name = [0u8; 32];
        assert_eq!(decode_beneficiary_name(&name, 0), "");
    }

    #[test]
    fn decodes_a_vault_and_beneficiary_list_into_a_snapshot() {
        use anchor_lang::AccountSerialize;
        use family_vault::state::{
            Beneficiary, BeneficiaryList, Vault, BENEFICIARY_NAME_LEN, MAX_BENEFICIARIES,
        };

        let vault = Vault {
            owner: Default::default(),
            vault_token_account: Default::default(),
            daily_limit: u64::MAX,
            spent_today: 0,
            last_reset_day: 0,
            is_pool: false,
            bump: 255,
            seed_owner: Default::default(),
        };
        let mut vault_bytes = Vec::new();
        vault.try_serialize(&mut vault_bytes).unwrap();

        let mut name = [0u8; BENEFICIARY_NAME_LEN];
        let src = "Внук Саша".as_bytes();
        name[..src.len()].copy_from_slice(src);
        let mut entries = [Beneficiary::default(); MAX_BENEFICIARIES];
        entries[0] = Beneficiary {
            address: Default::default(),
            name,
            name_len: src.len() as u8,
            added_at: 0,
            active_after: 0,
        };
        let list = BeneficiaryList {
            vault: Default::default(),
            count: 1,
            entries,
        };
        let mut list_bytes = Vec::new();
        list.try_serialize(&mut list_bytes).unwrap();

        let snapshot = decode_snapshot(&vault_bytes, &list_bytes, 3).unwrap();
        assert_eq!(snapshot.balance_text, "3 USDC");
        assert_eq!(snapshot.beneficiaries, vec![(1u8, "Внук Саша".to_string())]);
    }
}
