//! Everything the commands share. Cheap to clone (the heavy parts are
//! behind `Arc`), because blocking RPC work runs on another thread and
//! needs its own handle.

use std::path::PathBuf;
use std::sync::Arc;

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
}

impl AppState {
    pub fn me(&self) -> Pubkey {
        self.identity.pubkey()
    }

    pub fn pending_path(&self) -> PathBuf {
        self.data_dir.join(PENDING_FILE)
    }
}
