import { PublicKey } from '@solana/web3.js'

// This app runs exclusively on Solana Devnet.
export const CLUSTER = 'devnet' as const

export const RPC_URL = process.env.NEXT_PUBLIC_SOLANA_RPC || 'https://api.devnet.solana.com'

// Trellis program deployed on Devnet (matches declare_id! in programs/family-vault/src/lib.rs).
export const PROGRAM_ID = new PublicKey(
  process.env.NEXT_PUBLIC_PROGRAM_ID || '9ozRBpTmDJJzcmJhKEbfvsoe7qqo2584bus1rSMG4zwJ',
)

// Circle's official USDC mint on Devnet (SPL Token program, 6 decimals).
// Test USDC can be requested at https://faucet.circle.com (select Solana Devnet).
export const USDC_MINT = new PublicKey(
  process.env.NEXT_PUBLIC_USDC_MINT || '4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU',
)
export const USDC_DECIMALS = 6

// The dashboard creates pool vaults: the program enforces daily_limit on every
// withdrawal and routes over-limit requests to guardian approval.
export const DEFAULT_POOL_DAILY_LIMIT_USDC = 100

// Matches the seeds in programs/family-vault/src/instructions/initialize_vault.rs.
export function findPoolVaultPda(owner: PublicKey, programId: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync([Buffer.from('vault'), owner.toBuffer(), Buffer.from('pool')], programId)[0]
}

export const explorerTxUrl = (sig: string) => `https://explorer.solana.com/tx/${sig}?cluster=${CLUSTER}`
export const explorerAddressUrl = (addr: string) => `https://explorer.solana.com/address/${addr}?cluster=${CLUSTER}`
