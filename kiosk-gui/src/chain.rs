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

#[allow(dead_code)] // used starting in Task 3
pub fn format_amount(whole_units: u64) -> String {
    format!("{whole_units} USDC")
}

#[allow(dead_code)] // used starting in Task 4
pub fn parse_amount_to_base_units(whole_units: u64) -> u64 {
    whole_units * 10u64.pow(TOKEN_DECIMALS)
}

#[allow(dead_code)] // used starting in Task 3
pub fn decode_beneficiary_name(name: &[u8; 32], name_len: u8) -> String {
    let len = name_len as usize;
    String::from_utf8_lossy(&name[..len.min(32)]).into_owned()
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
}
