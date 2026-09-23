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
pub struct ProvisioningConfig {
    pub rpc_url: String,
    pub program_id: anchor_lang::prelude::Pubkey,
    pub mint: solana_pubkey::Pubkey,
}

#[derive(Debug)]
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

pub fn needs_setup(config_path: &Path) -> bool {
    !config_path.exists()
}

pub fn owner_keypair_path(config_dir: &Path) -> PathBuf {
    config_dir.join("owner-keypair.json")
}

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

/// Renders `data` as a black-on-white QR code raster with a quiet-zone
/// border (required for real-world scanability, not just a visual
/// nicety — most scanners fail without it). No anti-aliasing: QR
/// modules must stay pure black/white to stay machine-readable.
pub struct KioskConfigValues {
    pub rpc_url: String,
    pub keypair_path: String,
    pub program_id: String,
    pub mint: String,
    pub personal_vault: String,
    pub personal_beneficiary_list: String,
}

pub fn write_kiosk_config(
    path: &Path,
    values: &KioskConfigValues,
) -> Result<(), ProvisioningError> {
    let text = format!(
        "rpc_url = \"{}\"\n\
         keypair_path = \"{}\"\n\
         program_id = \"{}\"\n\
         mint = \"{}\"\n\
         personal_vault = \"{}\"\n\
         personal_beneficiary_list = \"{}\"\n\
         pool_vault = \"{}\"\n\
         pool_beneficiary_list = \"{}\"\n",
        values.rpc_url,
        values.keypair_path,
        values.program_id,
        values.mint,
        values.personal_vault,
        values.personal_beneficiary_list,
        // Placeholder pattern (design spec, "Фандинг и самостоятельная
        // инициализация" step 4): pool fields equal the personal
        // vault's own addresses until a family app creates a real pool
        // vault. Family Wallet will show the same data as My Wallet
        // until then — the same graceful-degradation path already
        // exercised manually with this exact placeholder trick.
        values.personal_vault,
        values.personal_beneficiary_list,
    );
    std::fs::write(path, text).map_err(ProvisioningError::Io)
}

pub fn render_qr(data: &str) -> slint::Image {
    let code = qrcode::QrCode::new(data.as_bytes()).expect("QR encoding failed");
    let width = code.width();
    let scale = 8usize;
    let border = 4usize;
    let img_size = (width + border * 2) * scale;

    let mut buffer =
        slint::SharedPixelBuffer::<slint::Rgb8Pixel>::new(img_size as u32, img_size as u32);
    let stride = buffer.width() as usize;
    let bytes = buffer.make_mut_bytes();

    for y in 0..img_size {
        for x in 0..img_size {
            let module_x = x / scale;
            let module_y = y / scale;
            let dark = module_x >= border
                && module_y >= border
                && module_x - border < width
                && module_y - border < width
                && code[(module_x - border, module_y - border)] == qrcode::Color::Dark;
            let v: u8 = if dark { 0 } else { 255 };
            let idx = (y * stride + x) * 3;
            bytes[idx] = v;
            bytes[idx + 1] = v;
            bytes[idx + 2] = v;
        }
    }
    slint::Image::from_rgb8(buffer)
}

/// Headroom above the raw rent-exemption sum, covering transaction fees
/// across a few submission attempts (each `initialize_vault` costs one
/// signature-verification fee; a handful of retries stay well under
/// this). Not rent — rent is computed live below, never guessed.
pub const MIN_FUNDING_LAMPORTS_BUFFER: u64 = 50_000;

pub fn compute_min_funding_lamports(
    rpc: &solana_client::rpc_client::RpcClient,
) -> Result<u64, crate::chain::ChainError> {
    let vault_rent = rpc
        .get_minimum_balance_for_rent_exemption(family_vault::state::Vault::SPACE)
        .map_err(crate::chain::map_client_error_pub)?;
    let list_rent = rpc
        .get_minimum_balance_for_rent_exemption(family_vault::state::BeneficiaryList::SPACE)
        .map_err(crate::chain::map_client_error_pub)?;
    let ata_rent = rpc
        .get_minimum_balance_for_rent_exemption(165) // spl_token::state::Account::LEN
        .map_err(crate::chain::map_client_error_pub)?;
    Ok(vault_rent + list_rent + ata_rent + MIN_FUNDING_LAMPORTS_BUFFER)
}

/// Self-initializes the device's own personal vault, paid for and signed
/// by the device's own owner keypair. Idempotent by existence check, not
/// by catching an "already exists" error: this first derives the vault
/// PDA and tries to read it; if that read succeeds, the vault already
/// exists (a prior attempt got the transaction through before a
/// crash/retry) and this returns immediately without sending anything —
/// cheap and unambiguous, versus parsing a program error string.
pub fn self_initialize_personal_vault(
    rpc: &solana_client::rpc_client::RpcClient,
    owner: &solana_keypair::Keypair,
    cfg: &ProvisioningConfig,
) -> Result<(solana_pubkey::Pubkey, solana_pubkey::Pubkey), crate::chain::ChainError> {
    use anchor_lang::{InstructionData, ToAccountMetas};
    use solana_signer::Signer;

    let owner_pubkey_client = owner.pubkey();
    let program_id_client =
        solana_pubkey::Pubkey::new_from_array(cfg.program_id.to_bytes());

    let (vault, _) = solana_pubkey::Pubkey::find_program_address(
        &[b"vault", owner_pubkey_client.as_ref(), b"personal"],
        &program_id_client,
    );
    let (beneficiary_list, _) = solana_pubkey::Pubkey::find_program_address(
        &[b"beneficiaries", vault.as_ref()],
        &program_id_client,
    );

    // Idempotency by existence check, not error-string matching — see
    // this function's doc comment.
    if rpc.get_account_data(&vault).is_ok() {
        return Ok((vault, beneficiary_list));
    }

    let vault_token_account =
        spl_associated_token_account::get_associated_token_address(&vault, &cfg.mint);

    let to_anchor = |p: solana_pubkey::Pubkey| {
        anchor_lang::prelude::Pubkey::new_from_array(p.to_bytes())
    };

    let data = family_vault::instruction::InitializeVault {
        is_pool: false,
        daily_limit: 0,
    }
    .data();
    let accounts = family_vault::accounts::InitializeVault {
        payer: to_anchor(owner_pubkey_client),
        owner: to_anchor(owner_pubkey_client),
        vault: to_anchor(vault),
        beneficiary_list: to_anchor(beneficiary_list),
        mint: to_anchor(cfg.mint),
        vault_token_account: to_anchor(vault_token_account),
        token_program: to_anchor(spl_token::ID),
        associated_token_program: to_anchor(spl_associated_token_account::ID),
        system_program: to_anchor(solana_pubkey::Pubkey::new_from_array([0u8; 32])),
    }
    .to_account_metas(None);

    let client_metas: Vec<solana_instruction::AccountMeta> = accounts
        .iter()
        .map(|m| solana_instruction::AccountMeta {
            pubkey: solana_pubkey::Pubkey::new_from_array(m.pubkey.to_bytes()),
            is_signer: m.is_signer,
            is_writable: m.is_writable,
        })
        .collect();

    let ix = solana_instruction::Instruction {
        program_id: program_id_client,
        accounts: client_metas,
        data,
    };

    let blockhash = rpc
        .get_latest_blockhash()
        .map_err(crate::chain::map_client_error_pub)?;
    let tx = solana_transaction::Transaction::new_signed_with_payer(
        &[ix],
        Some(&owner_pubkey_client),
        &[owner],
        blockhash,
    );
    rpc.send_and_confirm_transaction(&tx)
        .map_err(crate::chain::map_client_error_pub)?;

    Ok((vault, beneficiary_list))
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

    #[test]
    fn renders_a_qr_image_with_a_quiet_zone_border() {
        let image = render_qr("test-pubkey-string");
        let size = image.size();
        // Exact pixel size depends on the QR version chosen for this
        // input length, but it must be square and non-trivially sized
        // (a real QR code, not a 1x1 placeholder).
        assert_eq!(size.width, size.height);
        assert!(size.width > 100, "expected a real QR raster, got {size:?}");
    }

    #[test]
    fn written_config_round_trips_through_chain_load_config() {
        let path = std::env::temp_dir().join(format!(
            "kiosk-setup-test-write-config-{}.toml",
            std::process::id()
        ));
        let values = KioskConfigValues {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            keypair_path: "/etc/family-vault/owner-keypair.json".to_string(),
            program_id: "9ozRBpTmDJJzcmJhKEbfvsoe7qqo2584bus1rSMG4zwJ".to_string(),
            mint: "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".to_string(),
            personal_vault: "KziPLWvcZnYnt2ZhR7ACGHocBvgx2qX8jMExuxpFTQZ".to_string(),
            personal_beneficiary_list: "AbPLQhfRK16zi9vqUH1KutFDN8cU1bne4RXgRtKrEd3P".to_string(),
        };
        write_kiosk_config(&path, &values).unwrap();

        let loaded = crate::chain::load_config(&path).unwrap();
        assert_eq!(loaded.rpc_url, values.rpc_url);
        assert_eq!(loaded.keypair_path, values.keypair_path);
        // Pool fields must fall back to the personal vault's own
        // addresses (the placeholder pattern from the design spec's
        // "Фандинг и самостоятельная инициализация" step 4) until a
        // family app creates a real pool vault later.
        assert_eq!(
            loaded.pool_vault.to_string(),
            values.personal_vault,
            "pool_vault must default to personal_vault as a placeholder"
        );
        assert_eq!(
            loaded.pool_beneficiary_list.to_string(),
            values.personal_beneficiary_list
        );

        std::fs::remove_file(&path).unwrap();
    }

    // Requires a live RPC endpoint — run manually with
    // `cargo test --package kiosk-gui setup::tests::computes_a_real_minimum_funding_amount -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn computes_a_real_minimum_funding_amount() {
        let rpc = solana_client::rpc_client::RpcClient::new(
            "https://api.devnet.solana.com".to_string(),
        );
        let lamports = compute_min_funding_lamports(&rpc).unwrap();
        println!("min funding lamports: {lamports}");
        // Sanity bound: must cover at minimum {Vault + BeneficiaryList +
        // an ATA}'s rent, which on devnet today is on the order of a few
        // million lamports (a few thousandths of a SOL) — not zero, not
        // absurdly large (which would indicate a units bug, e.g. reading
        // SOL instead of lamports).
        assert!(lamports > 1_000_000, "suspiciously low: {lamports}");
        assert!(lamports < 100_000_000, "suspiciously high: {lamports}");
    }

    // Requires a live RPC endpoint AND a funded throwaway owner keypair
    // (needs at least compute_min_funding_lamports(&rpc) lamports before
    // running). Run manually with
    // `cargo test --package kiosk-gui setup::tests::self_init_is_idempotent -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn self_init_is_idempotent() {
        let rpc = solana_client::rpc_client::RpcClient::new(
            "https://api.devnet.solana.com".to_string(),
        );
        let owner = solana_keypair::read_keypair_file("/tmp/setup-test-owner.json")
            .expect("fund /tmp/setup-test-owner.json before running this test");
        let cfg = ProvisioningConfig {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            program_id: "9ozRBpTmDJJzcmJhKEbfvsoe7qqo2584bus1rSMG4zwJ"
                .parse()
                .unwrap(),
            mint: "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"
                .parse()
                .unwrap(),
        };

        let (vault1, list1) = self_initialize_personal_vault(&rpc, &owner, &cfg).unwrap();
        // Second call must not error and must return the SAME addresses
        // — this is the crash-mid-setup resumability the design spec
        // requires.
        let (vault2, list2) = self_initialize_personal_vault(&rpc, &owner, &cfg).unwrap();
        assert_eq!(vault1, vault2);
        assert_eq!(list1, list2);
    }
}
