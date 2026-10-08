'use client';

import { useConnection, useWallet } from '@solana/wallet-adapter-react';
import { PublicKey } from '@solana/web3.js';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { MOCK_MODE, findVaultPda, findBeneficiaryListPda, findApproversPda, FAMILY_VAULT_PROGRAM_ID } from '@/lib/constants';
import type {
  VaultAccount,
  BeneficiaryListAccount,
  TrustedApproversAccount,
  PendingWithdrawalAccount,
  PendingRotationAccount,
  VaultType,
} from '@/lib/types';
import {
  mockPersonalVault,
  mockPoolVault,
  mockBeneficiaryList,
  mockTrustedApprovers,
  mockPendingWithdrawals,
  mockPendingRotations,
  mockUsdcBalance,
  mockPersonalUsdcBalance,
} from '@/lib/mock-data';

// ─── Generic Fetch Hook ──────────────────────────────────────────────────────

interface FetchState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
  refetch: () => void;
}

function useFetchState<T>(
  fetcher: (() => Promise<T>) | null,
  mockData: T
): FetchState<T> {
  const [data, setData] = useState<T | null>(MOCK_MODE ? mockData : null);
  const [loading, setLoading] = useState(!MOCK_MODE);
  const [error, setError] = useState<string | null>(null);

  const fetch = useCallback(async () => {
    if (MOCK_MODE || !fetcher) {
      setData(mockData);
      setLoading(false);
      return;
    }
    try {
      setLoading(true);
      setError(null);
      const result = await fetcher();
      setData(result);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to fetch');
    } finally {
      setLoading(false);
    }
  }, [fetcher, mockData]);

  useEffect(() => {
    fetch();
  }, [fetch]);

  return { data, loading, error, refetch: fetch };
}

// ─── Vault Hook ──────────────────────────────────────────────────────────────

export function useVault(elderPubkey: PublicKey | null, vaultType: VaultType) {
  const { connection } = useConnection();
  const isPool = vaultType === 'pool';

  const fetcher = useMemo(() => {
    if (!elderPubkey) return null;
    return async (): Promise<VaultAccount> => {
      const [vaultPda] = findVaultPda(elderPubkey, isPool);
      const accountInfo = await connection.getAccountInfo(vaultPda);
      if (!accountInfo) throw new Error('Vault account not found');
      // TODO: Deserialize with Anchor IDL when program is deployed
      throw new Error('On-chain deserialization not yet implemented');
    };
  }, [elderPubkey, isPool, connection]);

  const mockData = isPool ? mockPoolVault : mockPersonalVault;
  return useFetchState(fetcher, mockData);
}

// ─── USDC Balance Hook ──────────────────────────────────────────────────────

export function useUsdcBalance(vaultType: VaultType) {
  const mockData = vaultType === 'pool' ? mockUsdcBalance : mockPersonalUsdcBalance;
  // In production: fetch SPL token account balance
  return useFetchState<bigint>(null, mockData);
}

// ─── Beneficiary List Hook ───────────────────────────────────────────────────

export function useBeneficiaryList(vaultPubkey: PublicKey | null) {
  const { connection } = useConnection();

  const fetcher = useMemo(() => {
    if (!vaultPubkey) return null;
    return async (): Promise<BeneficiaryListAccount> => {
      const [listPda] = findBeneficiaryListPda(vaultPubkey);
      const accountInfo = await connection.getAccountInfo(listPda);
      if (!accountInfo) throw new Error('BeneficiaryList not found');
      throw new Error('On-chain deserialization not yet implemented');
    };
  }, [vaultPubkey, connection]);

  return useFetchState(fetcher, mockBeneficiaryList);
}

// ─── Trusted Approvers Hook ─────────────────────────────────────────────────

export function useTrustedApprovers(vaultPubkey: PublicKey | null) {
  const { connection } = useConnection();

  const fetcher = useMemo(() => {
    if (!vaultPubkey) return null;
    return async (): Promise<TrustedApproversAccount> => {
      const [approversPda] = findApproversPda(vaultPubkey);
      const accountInfo = await connection.getAccountInfo(approversPda);
      if (!accountInfo) throw new Error('TrustedApprovers not found');
      throw new Error('On-chain deserialization not yet implemented');
    };
  }, [vaultPubkey, connection]);

  return useFetchState(fetcher, mockTrustedApprovers);
}

// ─── Pending Withdrawals Hook (uses getProgramAccounts + memcmp) ─────────────

export function usePendingWithdrawals(vaultPubkey: PublicKey | null) {
  const { connection } = useConnection();

  const fetcher = useMemo(() => {
    if (!vaultPubkey) return null;
    return async (): Promise<PendingWithdrawalAccount[]> => {
      // PendingWithdrawal seeds: ["request", vault_pubkey, nonce]
      // We use getProgramAccounts with a memcmp filter on the vault field
      // The vault field is the first field after the 8-byte discriminator
      const accounts = await connection.getProgramAccounts(
        FAMILY_VAULT_PROGRAM_ID,
        {
          filters: [
            { dataSize: 113 }, // Approximate size of PendingWithdrawal
            {
              memcmp: {
                offset: 8, // After 8-byte Anchor discriminator
                bytes: vaultPubkey.toBase58(),
              },
            },
          ],
        }
      );
      // TODO: Deserialize each account with Anchor IDL
      return accounts.map((a) => ({
        vault: vaultPubkey,
        amount: BigInt(0),
        to: PublicKey.default,
        requestedAt: BigInt(0),
        approvedBy: null,
        executed: false,
        publicKey: a.pubkey,
      }));
    };
  }, [vaultPubkey, connection]);

  return useFetchState(fetcher, mockPendingWithdrawals);
}

// ─── Pending Rotations Hook (uses getProgramAccounts + memcmp) ───────────────

export function usePendingRotations(vaultPubkey: PublicKey | null) {
  const { connection } = useConnection();

  const fetcher = useMemo(() => {
    if (!vaultPubkey) return null;
    return async (): Promise<PendingRotationAccount[]> => {
      // PendingRotation seeds: ["rotation", vault_pubkey, nonce]
      const accounts = await connection.getProgramAccounts(
        FAMILY_VAULT_PROGRAM_ID,
        {
          filters: [
            { dataSize: 145 }, // Approximate size of PendingRotation
            {
              memcmp: {
                offset: 8,
                bytes: vaultPubkey.toBase58(),
              },
            },
          ],
        }
      );
      return accounts.map((a) => ({
        vault: vaultPubkey,
        newOwner: PublicKey.default,
        requestedBy: PublicKey.default,
        requestedAt: BigInt(0),
        executesAt: BigInt(0),
        cancelled: false,
        publicKey: a.pubkey,
      }));
    };
  }, [vaultPubkey, connection]);

  return useFetchState(fetcher, mockPendingRotations);
}

// ─── Wallet Connection Hook ──────────────────────────────────────────────────

export function useWalletConnected() {
  const { connected, publicKey } = useWallet();
  return { connected, publicKey };
}
