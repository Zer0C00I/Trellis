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
/// `0600` permissions on unix automatically).
///
/// Creation is atomic and never overwrites: the key is first written to a
/// temporary file beside `path` and then hard-linked into place, which fails
/// if `path` already exists. A crash therefore leaves either no identity
/// file or a complete one, never a half-written key. If two processes race
/// on first launch, the loser loads the winner's key. Any problem other than
/// "file not found" (for example a permissions error) is returned as an
/// error rather than treated as "no identity yet".
pub fn generate_or_load_identity(path: &Path) -> Result<Keypair, IdentityError> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => return load(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(IdentityError::Io(e)),
    }

    let keypair = Keypair::new();
    let tmp = temporary_path(path);
    solana_keypair::write_keypair_file(&keypair, &tmp)
        .map_err(|e| IdentityError::Io(std::io::Error::other(e.to_string())))?;
    let linked = std::fs::hard_link(&tmp, path);
    let _ = std::fs::remove_file(&tmp);
    match linked {
        Ok(()) => Ok(keypair),
        // Someone else created it between our check and now: use theirs.
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => load(path),
        Err(e) => Err(IdentityError::Io(e)),
    }
}

fn load(path: &Path) -> Result<Keypair, IdentityError> {
    solana_keypair::read_keypair_file(path)
        .map_err(|e| IdentityError::Io(std::io::Error::other(e.to_string())))
}

fn temporary_path(path: &Path) -> std::path::PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".tmp-{}", std::process::id()));
    path.with_file_name(name)
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
        assert_ne!(
            identity.pubkey(),
            solana_pubkey::Pubkey::new_from_array([0u8; 32])
        );
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

    #[test]
    fn creation_leaves_no_temporary_file_behind() {
        let path = temp_path("clean");
        generate_or_load_identity(&path).unwrap();
        assert!(!temporary_path(&path).exists());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn an_existing_identity_is_never_overwritten() {
        let path = temp_path("keep");
        let original = generate_or_load_identity(&path).unwrap();
        let bytes_before = std::fs::read(&path).unwrap();
        let again = generate_or_load_identity(&path).unwrap();
        assert_eq!(original.pubkey(), again.pubkey());
        assert_eq!(bytes_before, std::fs::read(&path).unwrap());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn an_unreadable_location_is_an_error_not_a_fresh_identity() {
        // A path whose parent is a regular file can never be created or read.
        let blocker = temp_path("blocker");
        std::fs::write(&blocker, b"x").unwrap();
        let path = blocker.join("identity.json");
        assert!(matches!(
            generate_or_load_identity(&path),
            Err(IdentityError::Io(_))
        ));
        std::fs::remove_file(&blocker).unwrap();
    }
}
