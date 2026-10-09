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
/// `0600` permissions on unix automatically). Calling this again after a
/// crash that happened after the file was fully written just loads it. The
/// write is not atomic, so a crash in the middle of it can leave a partial
/// file, which the next call reports as an error rather than silently
/// replacing it.
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
