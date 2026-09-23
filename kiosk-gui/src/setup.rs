use std::fmt;
use std::path::{Path, PathBuf};

use solana_keypair::Keypair;

#[derive(serde::Deserialize)]
struct RawProvisioningConfig {
    rpc_url: String,
    program_id: String,
    mint: String,
}

#[derive(Debug)]
#[allow(dead_code)] // used starting in Task 6
pub struct ProvisioningConfig {
    pub rpc_url: String,
    pub program_id: anchor_lang::prelude::Pubkey,
    pub mint: solana_pubkey::Pubkey,
}

#[derive(Debug)]
#[allow(dead_code)] // used starting in Task 6
pub enum ProvisioningError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    BadPubkey { field: &'static str, value: String },
}

impl fmt::Display for ProvisioningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProvisioningError::Io(e) => write!(f, "could not read provisioning file: {e}"),
            ProvisioningError::Parse(e) => write!(f, "could not parse provisioning TOML: {e}"),
            ProvisioningError::BadPubkey { field, value } => {
                write!(f, "provisioning field '{field}' is not a valid pubkey: '{value}'")
            }
        }
    }
}

impl std::error::Error for ProvisioningError {}

#[allow(dead_code)] // used starting in Task 6
pub fn needs_setup(config_path: &Path) -> bool {
    !config_path.exists()
}

#[allow(dead_code)] // used starting in Task 6
pub fn owner_keypair_path(config_dir: &Path) -> PathBuf {
    config_dir.join("owner-keypair.json")
}

#[allow(dead_code)] // used starting in Task 6
pub fn load_provisioning_config(path: &Path) -> Result<ProvisioningConfig, ProvisioningError> {
    let text = std::fs::read_to_string(path).map_err(ProvisioningError::Io)?;
    let raw: RawProvisioningConfig = toml::from_str(&text).map_err(ProvisioningError::Parse)?;

    let program_id: anchor_lang::prelude::Pubkey =
        raw.program_id.parse().map_err(|_| ProvisioningError::BadPubkey {
            field: "program_id",
            value: raw.program_id.clone(),
        })?;
    let mint: solana_pubkey::Pubkey =
        raw.mint.parse().map_err(|_| ProvisioningError::BadPubkey {
            field: "mint",
            value: raw.mint.clone(),
        })?;

    Ok(ProvisioningConfig {
        rpc_url: raw.rpc_url,
        program_id,
        mint,
    })
}

#[allow(dead_code)] // used starting in Task 6
pub fn generate_or_load_owner_keypair(path: &Path) -> Result<Keypair, ProvisioningError> {
    if path.exists() {
        return solana_keypair::read_keypair_file(path)
            .map_err(|e| ProvisioningError::Io(std::io::Error::other(e.to_string())));
    }
    let keypair = Keypair::new();
    solana_keypair::write_keypair_file(&keypair, path)
        .map_err(|e| ProvisioningError::Io(std::io::Error::other(e.to_string())))?;
    Ok(keypair)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_signer::Signer;

    #[test]
    fn needs_setup_is_true_when_config_file_is_missing() {
        let path = std::env::temp_dir().join("kiosk-setup-test-missing-config.toml");
        let _ = std::fs::remove_file(&path);
        assert!(needs_setup(&path));
    }

    #[test]
    fn needs_setup_is_false_when_config_file_exists() {
        let path = std::env::temp_dir().join(format!(
            "kiosk-setup-test-existing-config-{}.toml",
            std::process::id()
        ));
        std::fs::write(&path, "anything").unwrap();
        assert!(!needs_setup(&path));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn loads_a_valid_provisioning_config() {
        let path = std::env::temp_dir().join(format!(
            "kiosk-setup-test-provisioning-{}.toml",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"
rpc_url = "https://api.devnet.solana.com"
program_id = "9ozRBpTmDJJzcmJhKEbfvsoe7qqo2584bus1rSMG4zwJ"
mint = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"
"#,
        )
        .unwrap();
        let cfg = load_provisioning_config(&path).unwrap();
        assert_eq!(cfg.rpc_url, "https://api.devnet.solana.com");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn rejects_provisioning_config_with_bad_mint() {
        let path = std::env::temp_dir().join(format!(
            "kiosk-setup-test-bad-mint-{}.toml",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"
rpc_url = "https://api.devnet.solana.com"
program_id = "9ozRBpTmDJJzcmJhKEbfvsoe7qqo2584bus1rSMG4zwJ"
mint = "not-a-pubkey"
"#,
        )
        .unwrap();
        let err = load_provisioning_config(&path).unwrap_err();
        assert!(matches!(err, ProvisioningError::BadPubkey { field: "mint", .. }));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn generates_a_new_keypair_when_none_exists_on_disk() {
        let path = std::env::temp_dir().join(format!(
            "kiosk-setup-test-keypair-new-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let kp = generate_or_load_owner_keypair(&path).unwrap();
        assert!(path.exists());
        // Loading again must return the SAME keypair, not a fresh one —
        // this is the power-loss-mid-setup resumability the design spec
        // requires: the already-shown QR pubkey must not change.
        let kp2 = generate_or_load_owner_keypair(&path).unwrap();
        assert_eq!(kp.pubkey(), kp2.pubkey());
        std::fs::remove_file(&path).unwrap();
    }
}
