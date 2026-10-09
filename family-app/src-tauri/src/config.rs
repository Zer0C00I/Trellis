//! Network settings. The app talks to Devnet only (the program is an
//! unaudited prototype); `TRELLIS_RPC_URL` overrides the endpoint for
//! development against a private node.

use solana_pubkey::Pubkey;

pub const DEFAULT_RPC_URL: &str = "https://api.devnet.solana.com";

/// Circle's Devnet USDC, the token every Trellis vault holds.
pub const DEVNET_USDC_MINT: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";

pub const IDENTITY_FILE: &str = "identity.json";
pub const PENDING_FILE: &str = "pending-transaction.json";

/// What the Devnet faucet is asked for: enough for a few hundred transactions.
pub const AIRDROP_LAMPORTS: u64 = 100_000_000;

pub fn rpc_url() -> String {
    std::env::var("TRELLIS_RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_string())
}

pub fn program_id() -> Pubkey {
    family_vault_client::to_client(family_vault::ID)
}

pub fn usdc_mint() -> Pubkey {
    DEVNET_USDC_MINT
        .parse()
        .expect("DEVNET_USDC_MINT is a valid base58 pubkey")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_usdc_mint_constant_parses() {
        assert_eq!(usdc_mint().to_string(), DEVNET_USDC_MINT);
    }

    #[test]
    fn the_program_id_is_the_declared_family_vault_id() {
        assert_eq!(program_id().to_bytes(), family_vault::ID.to_bytes());
    }
}
