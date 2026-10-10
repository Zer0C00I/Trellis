//! Everything the commands share. Cheap to clone (the heavy parts are
//! behind `Arc`), because blocking RPC work runs on another thread and
//! needs its own handle.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use family_app_core::history::HistoryCache;
use solana_client::rpc_client::RpcClient;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

use crate::config::PENDING_FILE;

#[derive(Clone)]
pub struct AppState {
    pub identity: Arc<Keypair>,
    pub rpc: Arc<RpcClient>,
    pub program_id: Pubkey,
    pub data_dir: PathBuf,
    /// Set while a money command runs, so a second one cannot start before
    /// the first has either finished or been recorded as unconfirmed.
    pub busy: Arc<AtomicBool>,
    /// Decoded transactions already seen, so a refresh only asks the node
    /// about new ones.
    pub history_cache: Arc<Mutex<HistoryCache>>,
}

/// Held for the whole of a money command; released on drop, also on panic.
pub struct BusyGuard(Arc<AtomicBool>);

impl BusyGuard {
    pub fn acquire(flag: &Arc<AtomicBool>) -> Option<BusyGuard> {
        flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| BusyGuard(Arc::clone(flag)))
    }
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl AppState {
    pub fn me(&self) -> Pubkey {
        self.identity.pubkey()
    }

    pub fn pending_path(&self) -> PathBuf {
        self.data_dir.join(PENDING_FILE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_acquire_fails_while_the_first_guard_lives() {
        let flag = Arc::new(AtomicBool::new(false));
        let first = BusyGuard::acquire(&flag);
        assert!(first.is_some());
        assert!(BusyGuard::acquire(&flag).is_none());
    }

    #[test]
    fn acquire_succeeds_again_after_the_guard_is_dropped() {
        let flag = Arc::new(AtomicBool::new(false));
        drop(BusyGuard::acquire(&flag));
        assert!(BusyGuard::acquire(&flag).is_some());
    }

    #[test]
    fn the_guard_is_released_when_the_holder_panics() {
        let flag = Arc::new(AtomicBool::new(false));
        let f = Arc::clone(&flag);
        let result = std::panic::catch_unwind(move || {
            let _guard = BusyGuard::acquire(&f).unwrap();
            panic!("boom");
        });
        assert!(result.is_err());
        assert!(BusyGuard::acquire(&flag).is_some());
    }
}
