//! Local name registry: lets `admin-cli` be driven by names ("бабушка",
//! "внук", "бабушкин кошелёк") instead of raw pubkeys/keypair paths.
//! Nobody using a family-facing panel should need to know what a pubkey
//! or a PDA is — see the discussion in this tool's `main.rs` doc comment.
//!
//! Stored as plain JSON at `--registry <path>` (default
//! `~/.family-vault/accounts.json`), created on first `register-person`/
//! `register-vault`/`create-vault` call if it doesn't exist yet.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use solana_keypair::{read_keypair_file, Keypair};
use solana_pubkey::Pubkey;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub people: BTreeMap<String, PersonEntry>,
    #[serde(default)]
    pub vaults: BTreeMap<String, VaultEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersonEntry {
    /// Path to a keypair file, when this tool can sign as this person
    /// (e.g. the vault owner, or a test beneficiary we control). Absent
    /// for a beneficiary who is only ever a receive-only address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keypair: Option<String>,
    /// Wallet pubkey, base58. Always present once resolved — a
    /// keypair-backed entry stores its derived pubkey here too, so the
    /// registry file is readable without re-deriving it each time.
    pub pubkey: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultEntry {
    pub pubkey: String,
    pub mint: String,
    /// Name of the `people` entry that owns this vault — resolving the
    /// owner's signing keypair for `send`/`add-beneficiary` goes through
    /// this, not a duplicated keypair path on the vault entry itself.
    pub owner: String,
}

pub fn default_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".family-vault").join("accounts.json")
}

pub fn load(path: &Path) -> Registry {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
            eprintln!("registry file {path:?} is not valid JSON: {e}");
            std::process::exit(1);
        }),
        Err(_) => Registry::default(),
    }
}

pub fn save(path: &Path, registry: &Registry) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| {
            eprintln!("could not create {parent:?}: {e}");
            std::process::exit(1);
        });
    }
    let text = serde_json::to_string_pretty(registry).unwrap();
    std::fs::write(path, text).unwrap_or_else(|e| {
        eprintln!("could not write registry to {path:?}: {e}");
        std::process::exit(1);
    });
}

impl Registry {
    pub fn person(&self, name: &str) -> &PersonEntry {
        self.people.get(name).unwrap_or_else(|| {
            eprintln!(
                "no person named '{name}' in the registry — register them first with \
                 `register-person --name {name} --keypair <path>` (or --pubkey <address> \
                 if this tool won't sign as them)"
            );
            std::process::exit(1);
        })
    }

    pub fn person_pubkey(&self, name: &str) -> Pubkey {
        self.person(name).pubkey.parse().unwrap_or_else(|_| {
            eprintln!("registry entry '{name}' has an invalid stored pubkey");
            std::process::exit(1);
        })
    }

    pub fn person_keypair(&self, name: &str) -> Keypair {
        let entry = self.person(name);
        let Some(path) = &entry.keypair else {
            eprintln!(
                "'{name}' has no keypair on file — this tool can't sign as them \
                 (they were registered with --pubkey only, receive-only)"
            );
            std::process::exit(1);
        };
        read_keypair_file(path).unwrap_or_else(|e| {
            eprintln!("'{name}' keypair ({path}): could not read: {e}");
            std::process::exit(1);
        })
    }

    pub fn vault(&self, name: &str) -> &VaultEntry {
        self.vaults.get(name).unwrap_or_else(|| {
            eprintln!(
                "no vault named '{name}' in the registry — create one first with \
                 `create-vault --name {name} --owner <person-name> --mint <address>`"
            );
            std::process::exit(1);
        })
    }

    pub fn vault_pubkey(&self, name: &str) -> Pubkey {
        self.vault(name).pubkey.parse().unwrap_or_else(|_| {
            eprintln!("registry entry '{name}' has an invalid stored vault pubkey");
            std::process::exit(1);
        })
    }
}
