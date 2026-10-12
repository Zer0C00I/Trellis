//! The one operation that was sent but whose outcome is not known yet.
//!
//! While this file exists, every command that moves money refuses to run
//! (see `commands::ensure_no_pending`): retrying with a fresh transaction
//! could pay twice. The file survives closing the app, so the rule holds
//! across restarts; it is removed only once the operation is settled.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::msg::Msg;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PendingTx {
    pub signature: String,
    pub last_valid_block_height: u64,
    /// What the user was doing, as a catalog key (`op.send`) ...
    #[serde(default)]
    pub what_key: Option<String>,
    /// ... and its parameters (`{"amount": "5 USDC"}`).
    #[serde(default)]
    pub what_params: Map<String, Value>,
    /// A sentence written by an earlier version of the app, which stored the
    /// words themselves. Read as a fallback and written empty, so an older
    /// app can still read a record this version made.
    #[serde(default)]
    pub what: String,
}

impl PendingTx {
    pub fn new(signature: String, last_valid_block_height: u64, what: &Msg) -> Self {
        Self {
            signature,
            last_valid_block_height,
            what_key: Some(what.key.clone()),
            what_params: what.params.clone(),
            what: String::new(),
        }
    }

    /// What the screen shows: the key if there is one, else the old sentence.
    pub fn what_msg(&self) -> Msg {
        match &self.what_key {
            Some(key) => Msg {
                key: key.clone(),
                params: self.what_params.clone(),
            },
            None => Msg::new("op.legacy").with("text", self.what.clone()),
        }
    }
}

/// Why the record could not be read. A corrupt file is told apart from an
/// I/O problem because only the former may be moved aside (`quarantine`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    Corrupt(String),
    Io(String),
}

pub fn load(path: &Path) -> Result<Option<PendingTx>, LoadError> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| LoadError::Corrupt(format!("could not parse {}: {e}", path.display()))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(LoadError::Io(format!(
            "could not read {}: {e}",
            path.display()
        ))),
    }
}

/// Moves an unreadable record aside as `<name>.corrupt-<unix seconds>` in
/// the same directory. Never deletes it: it may be the only trace of an
/// operation that moved money.
pub fn quarantine(path: &Path) -> Result<std::path::PathBuf, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let stem = path.file_name().unwrap_or_default().to_os_string();
    let candidate = |suffix: u32| {
        let mut name = stem.clone();
        name.push(format!(".corrupt-{}-{}", now.as_secs(), now.subsec_nanos()));
        if suffix > 0 {
            name.push(format!("-{suffix}"));
        }
        path.with_file_name(name)
    };
    // Unique even within one instant: never overwrite an earlier quarantine.
    let mut suffix = 0;
    let mut target = candidate(suffix);
    while target.exists() {
        suffix += 1;
        target = candidate(suffix);
    }
    std::fs::rename(path, &target)
        .map_err(|e| format!("could not move {} aside: {e}", path.display()))?;
    Ok(target)
}

/// Refuses to replace an existing record: an unresolved operation must
/// never be forgotten because a second one was started.
pub fn save(path: &Path, pending: &PendingTx) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(pending).map_err(|e| e.to_string())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("could not save {}: {e}", path.display()))?;
    file.write_all(&json)
        .and_then(|()| file.sync_all())
        .map_err(|e| format!("could not save {}: {e}", path.display()))
}

pub fn clear(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("could not remove {}: {e}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        std::env::temp_dir().join(format!("family-app-pending-{name}-{nanos}.json"))
    }

    fn sample() -> PendingTx {
        PendingTx::new(
            "5j7s".to_string(),
            123_456,
            &Msg::new("op.send").with("amount", "5 USDC"),
        )
    }

    #[test]
    fn a_record_from_an_earlier_version_keeps_its_sentence() {
        let old = r#"{"signature":"5j7s","last_valid_block_height":9,"what":"Отправка 5 USDC"}"#;
        let record: PendingTx = serde_json::from_str(old).unwrap();
        assert_eq!(
            record.what_msg(),
            Msg::new("op.legacy").with("text", "Отправка 5 USDC")
        );
    }

    #[test]
    fn a_new_record_names_what_it_was_and_stays_readable_by_the_old_format() {
        let record = sample();
        assert_eq!(
            record.what_msg(),
            Msg::new("op.send").with("amount", "5 USDC")
        );
        let json: Value = serde_json::to_value(&record).unwrap();
        assert_eq!(json["what"], "");
        assert_eq!(json["what_key"], "op.send");
    }

    #[test]
    fn nothing_saved_means_nothing_pending() {
        assert_eq!(load(&temp_path("none")).unwrap(), None);
    }

    #[test]
    fn a_saved_record_loads_back_unchanged() {
        let path = temp_path("roundtrip");
        save(&path, &sample()).unwrap();
        assert_eq!(load(&path).unwrap(), Some(sample()));
        clear(&path).unwrap();
        assert_eq!(load(&path).unwrap(), None);
    }

    #[test]
    fn a_second_save_cannot_overwrite_an_unresolved_one() {
        let path = temp_path("noclobber");
        save(&path, &sample()).unwrap();
        let other = PendingTx {
            signature: "other".to_string(),
            ..sample()
        };
        assert!(save(&path, &other).is_err());
        assert_eq!(load(&path).unwrap(), Some(sample()));
        clear(&path).unwrap();
    }

    #[test]
    fn a_corrupt_record_is_an_error_not_silently_ignored() {
        let path = temp_path("corrupt");
        std::fs::write(&path, b"not json").unwrap();
        assert!(load(&path).is_err());
        clear(&path).unwrap();
    }

    #[test]
    fn clearing_twice_is_fine() {
        let path = temp_path("twice");
        clear(&path).unwrap();
        clear(&path).unwrap();
    }

    #[test]
    fn load_reports_garbage_as_corrupt_not_io() {
        let path = temp_path("garbage");
        std::fs::write(&path, b"{ nope").unwrap();
        assert!(matches!(load(&path), Err(LoadError::Corrupt(_))));
        clear(&path).unwrap();
    }

    #[test]
    fn quarantine_renames_and_keeps_the_bytes() {
        let path = temp_path("quarantine");
        std::fs::write(&path, b"precious garbage").unwrap();
        let moved = quarantine(&path).unwrap();
        assert!(!path.exists());
        assert!(moved.to_string_lossy().contains(".corrupt-"));
        assert_eq!(moved.parent(), path.parent());
        assert_eq!(std::fs::read(&moved).unwrap(), b"precious garbage");
        assert_eq!(load(&path).unwrap(), None);
        std::fs::remove_file(&moved).unwrap();
    }

    #[test]
    fn two_quick_quarantines_give_two_files_and_lose_nothing() {
        let path = temp_path("twice-quarantined");
        std::fs::write(&path, b"first").unwrap();
        let one = quarantine(&path).unwrap();
        std::fs::write(&path, b"second").unwrap();
        let two = quarantine(&path).unwrap();
        assert_ne!(one, two);
        assert_eq!(std::fs::read(&one).unwrap(), b"first");
        assert_eq!(std::fs::read(&two).unwrap(), b"second");
        std::fs::remove_file(&one).unwrap();
        std::fs::remove_file(&two).unwrap();
    }

    #[test]
    fn quarantining_a_missing_file_is_an_error() {
        assert!(quarantine(&temp_path("missing")).is_err());
    }
}
