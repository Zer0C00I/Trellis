//! The app key's own SOL: balance for the "you need SOL for fees" hint, and
//! a Devnet-only faucet request. Every transaction the app sends is paid
//! for with SOL from this key, so a brand-new identity cannot do anything
//! until it has some.

use std::time::{Duration, Instant};

use solana_client::rpc_client::RpcClient;
use solana_pubkey::Pubkey;

use crate::error::{map_client_error, ActionError};

/// Below this the app tells the user to add SOL before doing anything that
/// costs a fee (about 0.01 SOL covers a few dozen transactions).
pub const LOW_SOL_LAMPORTS: u64 = 10_000_000;

const AIRDROP_WAIT: Duration = Duration::from_secs(30);
const AIRDROP_POLL: Duration = Duration::from_secs(1);

pub fn sol_balance(rpc: &RpcClient, address: &Pubkey) -> Result<u64, ActionError> {
    rpc.get_balance(address).map_err(map_client_error)
}

/// Requests `lamports` from the Devnet faucet and waits until the funds
/// arrive. Devnet only: on any other cluster the faucet call is refused by
/// the node. The public faucet is rate limited, so a refusal is common.
pub fn request_devnet_airdrop(
    rpc: &RpcClient,
    address: &Pubkey,
    lamports: u64,
) -> Result<(), ActionError> {
    let signature = rpc
        .request_airdrop(address, lamports)
        .map_err(map_client_error)?;
    let deadline = Instant::now() + AIRDROP_WAIT;
    loop {
        match rpc.confirm_transaction(&signature) {
            Ok(true) => return Ok(()),
            Ok(false) => {}
            Err(e) => return Err(map_client_error(e)),
        }
        if Instant::now() >= deadline {
            return Err(ActionError::Rejected(
                "the faucet request did not complete in time".to_string(),
            ));
        }
        std::thread::sleep(AIRDROP_POLL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_low_sol_threshold_is_a_sensible_fraction_of_one_sol() {
        const { assert!(LOW_SOL_LAMPORTS > 0 && LOW_SOL_LAMPORTS < 1_000_000_000) };
    }

    // Needs a Devnet endpoint and the public faucet (rate limited). Run with
    // `DEVNET_AIRDROP_ADDRESS=<pubkey> cargo test -p family-app-core wallet::tests::airdrop -- --ignored --nocapture`
    #[test]
    #[ignore = "needs Devnet and the public faucet"]
    fn airdrop_arrives() {
        let address: Pubkey = std::env::var("DEVNET_AIRDROP_ADDRESS")
            .expect("set DEVNET_AIRDROP_ADDRESS")
            .parse()
            .unwrap();
        let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());
        let before = sol_balance(&rpc, &address).unwrap();
        request_devnet_airdrop(&rpc, &address, 100_000_000).unwrap();
        assert!(sol_balance(&rpc, &address).unwrap() > before);
    }
}
