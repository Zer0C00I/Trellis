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

/// How many `confirm_transaction` errors in a row the wait tolerates; the
/// next one ends it. The airdrop may well have landed despite a flaky poll.
const MAX_CONSECUTIVE_POLL_ERRORS: u32 = 3;

/// The count of consecutive poll errors after one more poll: a success
/// resets it, an error adds one. `None` once the budget is exhausted (the
/// error just seen is the `MAX_CONSECUTIVE_POLL_ERRORS + 1`-th in a row).
fn next_error_streak(streak: u32, poll_failed: bool) -> Option<u32> {
    if !poll_failed {
        return Some(0);
    }
    let streak = streak + 1;
    (streak <= MAX_CONSECUTIVE_POLL_ERRORS).then_some(streak)
}

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
    let mut error_streak = 0;
    loop {
        match rpc.confirm_transaction(&signature) {
            Ok(true) => return Ok(()),
            Ok(false) => error_streak = 0,
            Err(e) => match next_error_streak(error_streak, true) {
                Some(streak) => error_streak = streak,
                None => return Err(map_client_error(e)),
            },
        }
        if Instant::now() >= deadline {
            return Err(ActionError::Rejected(
                "the faucet request did not finish in time; the SOL may still arrive, check the balance"
                    .to_string(),
            ));
        }
        std::thread::sleep(AIRDROP_POLL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_poll_errors_in_a_row_are_tolerated_the_fourth_is_not() {
        let mut streak = 0;
        for expected in 1..=3 {
            streak = next_error_streak(streak, true).expect("within budget");
            assert_eq!(streak, expected);
        }
        assert_eq!(next_error_streak(streak, true), None);
    }

    #[test]
    fn a_successful_poll_resets_the_error_streak() {
        let mut streak = 0;
        for _ in 0..3 {
            streak = next_error_streak(streak, true).unwrap();
        }
        streak = next_error_streak(streak, false).unwrap();
        assert_eq!(streak, 0);
        // A full budget is available again.
        for _ in 0..3 {
            streak = next_error_streak(streak, true).unwrap();
        }
        assert_eq!(next_error_streak(streak, true), None);
    }

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
