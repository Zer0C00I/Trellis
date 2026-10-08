import { PublicKey } from '@solana/web3.js';
import type {
  VaultAccount,
  BeneficiaryListAccount,
  TrustedApproversAccount,
  PendingWithdrawalAccount,
  PendingRotationAccount,
} from './types';
import { U64_MAX } from './constants';

// ─── Mock Keys ───────────────────────────────────────────────────────────────
const makeKey = (seed: number) => {
  const arr = new Uint8Array(32);
  arr[0] = seed;
  return new PublicKey(arr);
};

const MOCK_ELDER = makeKey(1);
const MOCK_VAULT_PERSONAL = makeKey(2);
const MOCK_VAULT_POOL = makeKey(3);
const MOCK_TOKEN_ACCOUNT = makeKey(4);

const MOCK_BENEFICIARIES: PublicKey[] = [
  makeKey(10),
  makeKey(11),
  makeKey(12),
  makeKey(13),
];

const MOCK_APPROVERS: PublicKey[] = [
  makeKey(20),
  makeKey(21),
  makeKey(22),
];

const nowSec = Math.floor(Date.now() / 1000);
const hoursAgo = (h: number) => BigInt(nowSec - h * 3600);
const hoursFromNow = (h: number) => BigInt(nowSec + h * 3600);

// ─── Mock Vaults ─────────────────────────────────────────────────────────────
export const mockPersonalVault: VaultAccount = {
  owner: MOCK_ELDER,
  vaultTokenAccount: MOCK_TOKEN_ACCOUNT,
  dailyLimit: U64_MAX,
  spentToday: BigInt(0),
  lastResetDay: BigInt(Math.floor(nowSec / 86400)),
  isPool: false,
  bump: 255,
};

export const mockPoolVault: VaultAccount = {
  owner: MOCK_ELDER,
  vaultTokenAccount: MOCK_TOKEN_ACCOUNT,
  dailyLimit: BigInt(500_000_000), // 500 USDC
  spentToday: BigInt(125_000_000), // 125 USDC spent today
  lastResetDay: BigInt(Math.floor(nowSec / 86400)),
  isPool: true,
  bump: 254,
};

// ─── Mock Beneficiary List ───────────────────────────────────────────────────
export const mockBeneficiaryList: BeneficiaryListAccount = {
  vault: MOCK_VAULT_POOL,
  entries: [
    {
      address: MOCK_BENEFICIARIES[0],
      name: 'Pharmacy — CVS',
      addedAt: hoursAgo(120),
      activeAfter: hoursAgo(72), // active (past cooldown)
    },
    {
      address: MOCK_BENEFICIARIES[1],
      name: 'Grocery — Walmart',
      addedAt: hoursAgo(96),
      activeAfter: hoursAgo(48), // active
    },
    {
      address: MOCK_BENEFICIARIES[2],
      name: 'Grandchild — Maria',
      addedAt: hoursAgo(24),
      activeAfter: hoursFromNow(24), // still in cooldown: 24h remaining
    },
    {
      address: MOCK_BENEFICIARIES[3],
      name: 'Doctor — Dr. Chen',
      addedAt: hoursAgo(6),
      activeAfter: hoursFromNow(42), // cooldown: 42h remaining
    },
  ],
};

// ─── Mock Trusted Approvers ──────────────────────────────────────────────────
export const mockTrustedApprovers: TrustedApproversAccount = {
  vault: MOCK_VAULT_POOL,
  entries: [MOCK_APPROVERS[0], MOCK_APPROVERS[1], MOCK_APPROVERS[2]],
};

// ─── Mock Pending Withdrawals ────────────────────────────────────────────────
export const mockPendingWithdrawals: PendingWithdrawalAccount[] = [
  {
    vault: MOCK_VAULT_POOL,
    amount: BigInt(750_000_000), // 750 USDC — exceeds daily limit
    to: MOCK_BENEFICIARIES[0],
    requestedAt: hoursAgo(3),
    approvedBy: null,
    executed: false,
    publicKey: makeKey(30),
  },
  {
    vault: MOCK_VAULT_POOL,
    amount: BigInt(200_000_000), // 200 USDC
    to: MOCK_BENEFICIARIES[1],
    requestedAt: hoursAgo(12),
    approvedBy: MOCK_APPROVERS[0],
    executed: true,
    publicKey: makeKey(31),
  },
];

// ─── Mock Pending Rotations ──────────────────────────────────────────────────
export const mockPendingRotations: PendingRotationAccount[] = [
  {
    vault: MOCK_VAULT_POOL,
    newOwner: makeKey(40),
    requestedBy: MOCK_APPROVERS[1],
    requestedAt: hoursAgo(48),
    executesAt: hoursFromNow(24), // 24h remaining of 72h timelock
    cancelled: false,
    publicKey: makeKey(41),
  },
];

// ─── Mock USDC Balance ───────────────────────────────────────────────────────
export const mockUsdcBalance = BigInt(2_347_500_000); // 2,347.50 USDC
export const mockPersonalUsdcBalance = BigInt(15_820_000_000); // 15,820.00 USDC
