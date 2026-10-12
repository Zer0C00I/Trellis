'use client'

import { useConnection, useWallet } from '@solana/wallet-adapter-react'
import { PublicKey, Transaction, TransactionInstruction } from '@solana/web3.js'
import { useCallback, useState } from 'react'
import { toast } from 'sonner'
import { MOCK_MODE, FAMILY_VAULT_PROGRAM_ID } from '@/lib/constants'

interface TxState {
  loading: boolean
  error: string | null
}

function useTransactionExecutor() {
  const { connection } = useConnection()
  const { publicKey, sendTransaction } = useWallet()
  const [state, setState] = useState<TxState>({ loading: false, error: null })

  const execute = useCallback(
    async (instruction: TransactionInstruction, description: string): Promise<string | null> => {
      if (MOCK_MODE) {
        setState({ loading: true, error: null })
        toast.loading(`${description}...`, { id: description })

        // Simulate network delay
        await new Promise((r) => setTimeout(r, 1500))

        const mockSig = 'mock_' + Math.random().toString(36).slice(2, 10)
        toast.success(`${description} — confirmed`, {
          id: description,
          description: `Signature: ${mockSig}`,
          duration: 5000,
        })
        setState({ loading: false, error: null })
        return mockSig
      }

      if (!publicKey) {
        toast.error('Wallet not connected')
        return null
      }

      try {
        setState({ loading: true, error: null })
        toast.loading(`${description}...`, { id: description })

        const tx = new Transaction().add(instruction)
        const sig = await sendTransaction(tx, connection)

        toast.loading(`Confirming ${description}...`, { id: description })
        await connection.confirmTransaction(sig, 'confirmed')

        toast.success(`${description} — confirmed`, {
          id: description,
          description: `Signature: ${sig.slice(0, 16)}...`,
          duration: 5000,
        })
        setState({ loading: false, error: null })
        return sig
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : 'Transaction failed'
        toast.error(`${description} failed`, {
          id: description,
          description: msg,
          duration: 8000,
        })
        setState({ loading: false, error: msg })
        return null
      }
    },
    [publicKey, connection, sendTransaction],
  )

  return { execute, ...state }
}

// ─── Add Beneficiary ─────────────────────────────────────────────────────────

export function useAddBeneficiary() {
  const { execute, loading, error } = useTransactionExecutor()

  const addBeneficiary = useCallback(
    async (name: string, address: PublicKey, beneficiaryListPda: PublicKey, vaultPda: PublicKey) => {
      // Build the instruction
      // In production: use Anchor's program.methods.addBeneficiary(name, address)...
      const ix = new TransactionInstruction({
        programId: FAMILY_VAULT_PROGRAM_ID,
        keys: [
          { pubkey: beneficiaryListPda, isSigner: false, isWritable: true },
          { pubkey: vaultPda, isSigner: false, isWritable: false },
          { pubkey: PublicKey.default, isSigner: true, isWritable: false }, // authority placeholder
        ],
        data: Buffer.from([]), // Placeholder — Anchor will encode properly
      })

      return execute(ix, `Adding beneficiary "${name}"`)
    },
    [execute],
  )

  return { addBeneficiary, loading, error }
}

// ─── Approve Withdrawal ──────────────────────────────────────────────────────

export function useApproveWithdrawal() {
  const { execute, loading, error } = useTransactionExecutor()

  const approveWithdrawal = useCallback(
    async (requestPubkey: PublicKey) => {
      const ix = new TransactionInstruction({
        programId: FAMILY_VAULT_PROGRAM_ID,
        keys: [
          { pubkey: requestPubkey, isSigner: false, isWritable: true },
          { pubkey: PublicKey.default, isSigner: true, isWritable: false },
        ],
        data: Buffer.from([]),
      })

      return execute(ix, 'Approving withdrawal')
    },
    [execute],
  )

  return { approveWithdrawal, loading, error }
}

// ─── Deposit USDC ────────────────────────────────────────────────────────────

export function useDeposit() {
  const { execute, loading, error } = useTransactionExecutor()

  const deposit = useCallback(
    async (amount: bigint, vaultPda: PublicKey) => {
      const ix = new TransactionInstruction({
        programId: FAMILY_VAULT_PROGRAM_ID,
        keys: [
          { pubkey: vaultPda, isSigner: false, isWritable: false },
          { pubkey: PublicKey.default, isSigner: false, isWritable: true }, // vault token account
          { pubkey: PublicKey.default, isSigner: false, isWritable: true }, // depositor token account
          { pubkey: PublicKey.default, isSigner: true, isWritable: false }, // depositor
        ],
        data: Buffer.from([]),
      })

      const usdcAmount = (Number(amount) / 1_000_000).toFixed(2)
      return execute(ix, `Depositing $${usdcAmount} USDC`)
    },
    [execute],
  )

  return { deposit, loading, error }
}

// ─── Request Owner Rotation ──────────────────────────────────────────────────

export function useRequestRotation() {
  const { execute, loading, error } = useTransactionExecutor()

  const requestRotation = useCallback(
    async (newOwner: PublicKey, vaultPda: PublicKey) => {
      const ix = new TransactionInstruction({
        programId: FAMILY_VAULT_PROGRAM_ID,
        keys: [
          { pubkey: PublicKey.default, isSigner: false, isWritable: true }, // pendingRotation PDA
          { pubkey: vaultPda, isSigner: false, isWritable: false },
          { pubkey: PublicKey.default, isSigner: true, isWritable: true }, // requester
        ],
        data: Buffer.from([]),
      })

      return execute(ix, 'Requesting owner rotation (72h timelock)')
    },
    [execute],
  )

  return { requestRotation, loading, error }
}

// ─── Execute Owner Rotation ──────────────────────────────────────────────────

export function useExecuteRotation() {
  const { execute, loading, error } = useTransactionExecutor()

  const executeRotation = useCallback(
    async (rotationPubkey: PublicKey, vaultPda: PublicKey) => {
      const ix = new TransactionInstruction({
        programId: FAMILY_VAULT_PROGRAM_ID,
        keys: [
          { pubkey: rotationPubkey, isSigner: false, isWritable: true },
          { pubkey: vaultPda, isSigner: false, isWritable: true },
          { pubkey: PublicKey.default, isSigner: true, isWritable: false }, // executor
        ],
        data: Buffer.from([]),
      })

      return execute(ix, 'Executing owner rotation')
    },
    [execute],
  )

  return { executeRotation, loading, error }
}

// ─── Cancel Owner Rotation ───────────────────────────────────────────────────

export function useCancelRotation() {
  const { execute, loading, error } = useTransactionExecutor()

  const cancelRotation = useCallback(
    async (rotationPubkey: PublicKey, vaultPda: PublicKey) => {
      const ix = new TransactionInstruction({
        programId: FAMILY_VAULT_PROGRAM_ID,
        keys: [
          { pubkey: rotationPubkey, isSigner: false, isWritable: true },
          { pubkey: vaultPda, isSigner: false, isWritable: false },
          { pubkey: PublicKey.default, isSigner: true, isWritable: false }, // authority
        ],
        data: Buffer.from([]),
      })

      return execute(ix, 'Cancelling owner rotation')
    },
    [execute],
  )

  return { cancelRotation, loading, error }
}
