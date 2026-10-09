import { PublicKey } from '@solana/web3.js';
import { PROGRAM_ID, USDC_MINT, USDC_DECIMALS } from '@/config/solana';

// ─── Program & Token Constants ───────────────────────────────────────────────
// Single source of truth lives in config/solana.ts; these are aliases kept so
// existing imports keep working.
export const FAMILY_VAULT_PROGRAM_ID = PROGRAM_ID;
export const USDC_DEVNET_MINT = USDC_MINT;
export { USDC_DECIMALS };

// ─── Mock Mode ───────────────────────────────────────────────────────────────
// Set to true for demo/development before the Anchor program is deployed.
// When true, all hooks return realistic mock data and actions show simulated toasts.
export const MOCK_MODE = true;

// ─── PDA Seeds ───────────────────────────────────────────────────────────────
export const VAULT_SEED = 'vault';
export const BENEFICIARY_SEED = 'beneficiaries';
export const APPROVERS_SEED = 'approvers';
export const REQUEST_SEED = 'request';
export const ROTATION_SEED = 'rotation';

// ─── Timing Constants ────────────────────────────────────────────────────────
export const BENEFICIARY_COOLDOWN_HOURS = 48;
export const ROTATION_TIMELOCK_HOURS = 72;
export const SECONDS_PER_DAY = 86400;

// ─── Limits ──────────────────────────────────────────────────────────────────
export const MAX_BENEFICIARIES = 10;
export const MAX_APPROVERS = 5;
export const U64_MAX = BigInt('18446744073709551615');

// ─── PDA Derivation Helpers ──────────────────────────────────────────────────
export function findVaultPda(
  elderPubkey: PublicKey,
  isPool: boolean
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from(VAULT_SEED),
      elderPubkey.toBuffer(),
      Buffer.from(isPool ? 'pool' : 'personal'),
    ],
    FAMILY_VAULT_PROGRAM_ID
  );
}

export function findBeneficiaryListPda(
  vaultPubkey: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(BENEFICIARY_SEED), vaultPubkey.toBuffer()],
    FAMILY_VAULT_PROGRAM_ID
  );
}

export function findApproversPda(
  vaultPubkey: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(APPROVERS_SEED), vaultPubkey.toBuffer()],
    FAMILY_VAULT_PROGRAM_ID
  );
}
