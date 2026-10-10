import { PublicKey } from '@solana/web3.js'

// ─── Program & Token Constants ───────────────────────────────────────────────
export const FAMILY_VAULT_PROGRAM_ID = new PublicKey(
  // Replace with actual deployed program ID
  '9ozRBpTmDJJzcmJhKEbfvsoe7qqo2584bus1rSMG4zwJ',
)

export const USDC_DEVNET_MINT = new PublicKey('4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU')

export const USDC_DECIMALS = 6

// ─── Mock Mode ───────────────────────────────────────────────────────────────
// Set to true for demo/development before the Anchor program is deployed.
// When true, all hooks return realistic mock data and actions show simulated toasts.
export const MOCK_MODE = false

// ─── PDA Seeds ───────────────────────────────────────────────────────────────
export const VAULT_SEED = 'vault'
export const BENEFICIARY_SEED = 'beneficiaries'
export const APPROVERS_SEED = 'approvers'
export const REQUEST_SEED = 'request'
export const ROTATION_SEED = 'rotation'

// ─── Timing Constants ────────────────────────────────────────────────────────
export const BENEFICIARY_COOLDOWN_HOURS = 48
export const ROTATION_TIMELOCK_HOURS = 72
export const SECONDS_PER_DAY = 86400

// ─── Limits ──────────────────────────────────────────────────────────────────
export const MAX_BENEFICIARIES = 10
export const MAX_APPROVERS = 5
export const U64_MAX = BigInt('18446744073709551615')

// ─── PDA Derivation Helpers ──────────────────────────────────────────────────
export function findVaultPda(elderPubkey: PublicKey, isPool: boolean): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(VAULT_SEED), elderPubkey.toBuffer(), Buffer.from(isPool ? 'pool' : 'personal')],
    FAMILY_VAULT_PROGRAM_ID,
  )
}

export function findBeneficiaryListPda(vaultPubkey: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(BENEFICIARY_SEED), vaultPubkey.toBuffer()],
    FAMILY_VAULT_PROGRAM_ID,
  )
}

export function findApproversPda(vaultPubkey: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(APPROVERS_SEED), vaultPubkey.toBuffer()],
    FAMILY_VAULT_PROGRAM_ID,
  )
}
