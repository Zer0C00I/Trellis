import { PublicKey } from '@solana/web3.js';

// ─── On-chain Account Types ─────────────────────────────────────────────────

export interface VaultAccount {
  owner: PublicKey;
  vaultTokenAccount: PublicKey;
  dailyLimit: bigint;
  spentToday: bigint;
  lastResetDay: bigint;
  isPool: boolean;
  bump: number;
}

export interface Beneficiary {
  address: PublicKey;
  name: string;
  addedAt: bigint; // unix timestamp
  activeAfter: bigint; // addedAt + 48h
}

export interface BeneficiaryListAccount {
  vault: PublicKey;
  entries: Beneficiary[];
}

export interface TrustedApproversAccount {
  vault: PublicKey;
  entries: PublicKey[];
}

export interface PendingWithdrawalAccount {
  vault: PublicKey;
  amount: bigint;
  to: PublicKey;
  requestedAt: bigint;
  approvedBy: PublicKey | null;
  executed: boolean;
  // Derived client-side
  publicKey?: PublicKey;
}

export interface PendingRotationAccount {
  vault: PublicKey;
  newOwner: PublicKey;
  requestedBy: PublicKey;
  requestedAt: bigint;
  executesAt: bigint;
  cancelled: boolean;
  // Derived client-side
  publicKey?: PublicKey;
}

// ─── UI-Specific Types ──────────────────────────────────────────────────────

export type VaultType = 'personal' | 'pool';

export interface BeneficiaryStatus {
  isActive: boolean;
  remainingMs: number; // 0 if active
  label: string;
}

export interface WithdrawalStatus {
  isPending: boolean;
  isApproved: boolean;
  isExecuted: boolean;
  label: string;
}

export interface RotationStatus {
  isReady: boolean; // now >= executesAt
  isCancelled: boolean;
  remainingMs: number;
  label: string;
}

// ─── Helper Functions ────────────────────────────────────────────────────────

export function getBeneficiaryStatus(b: Beneficiary): BeneficiaryStatus {
  const now = Date.now();
  const activeAfterMs = Number(b.activeAfter) * 1000;
  const remaining = activeAfterMs - now;

  if (remaining <= 0) {
    return { isActive: true, remainingMs: 0, label: 'Active' };
  }

  const hours = Math.floor(remaining / 3_600_000);
  const minutes = Math.floor((remaining % 3_600_000) / 60_000);
  return {
    isActive: false,
    remainingMs: remaining,
    label: `Cooldown: ${hours}h ${minutes}m`,
  };
}

export function getWithdrawalStatus(
  w: PendingWithdrawalAccount
): WithdrawalStatus {
  if (w.executed) {
    return {
      isPending: false,
      isApproved: true,
      isExecuted: true,
      label: 'Executed',
    };
  }
  if (w.approvedBy) {
    return {
      isPending: false,
      isApproved: true,
      isExecuted: false,
      label: 'Approved',
    };
  }
  return {
    isPending: true,
    isApproved: false,
    isExecuted: false,
    label: 'Awaiting Approval',
  };
}

export function getRotationStatus(
  r: PendingRotationAccount
): RotationStatus {
  if (r.cancelled) {
    return {
      isReady: false,
      isCancelled: true,
      remainingMs: 0,
      label: 'Cancelled',
    };
  }

  const now = Date.now();
  const executesAtMs = Number(r.executesAt) * 1000;
  const remaining = executesAtMs - now;

  if (remaining <= 0) {
    return {
      isReady: true,
      isCancelled: false,
      remainingMs: 0,
      label: 'Ready to Execute',
    };
  }

  const hours = Math.floor(remaining / 3_600_000);
  const minutes = Math.floor((remaining % 3_600_000) / 60_000);
  return {
    isReady: false,
    isCancelled: false,
    remainingMs: remaining,
    label: `Timelock: ${hours}h ${minutes}m remaining`,
  };
}

export function formatUsdc(lamports: bigint): string {
  const val = Number(lamports) / 1_000_000;
  return val.toLocaleString('en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

export function truncateAddress(address: string, chars = 4): string {
  if (address.length <= chars * 2 + 3) return address;
  return `${address.slice(0, chars)}...${address.slice(-chars)}`;
}
