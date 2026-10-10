'use client'

import { useEffect, useState } from 'react'
import { PublicKey } from '@solana/web3.js'
import { useAnchorProvider } from '../../config/useAnchorProvider'
import { AddressChip } from './AddressChip'

/** Read-only list of the vault's trusted approvers (PDA: ["approvers", vault]). */
export function GuardianList({
  vaultPda,
  refreshCounter,
  onLoaded,
}: {
  vaultPda: PublicKey
  refreshCounter: number
  onLoaded?: (count: number) => void
}) {
  const { program } = useAnchorProvider()
  const [guardians, setGuardians] = useState<string[] | null>(null)

  useEffect(() => {
    if (!program) return
    let cancelled = false
    const load = async () => {
      try {
        const [approversPda] = PublicKey.findProgramAddressSync(
          [Buffer.from('approvers'), vaultPda.toBuffer()],
          program.programId,
        )
        const account = await program.account.trustedApprovers.fetchNullable(approversPda)
        const list = account ? account.entries.slice(0, account.count).map((k) => k.toBase58()) : []
        if (!cancelled) {
          setGuardians(list)
          onLoaded?.(list.length)
        }
      } catch (err) {
        console.error('Failed to fetch guardians:', err)
        if (!cancelled) setGuardians([])
      }
    }
    load()
    return () => {
      cancelled = true
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [program, vaultPda, refreshCounter])

  if (guardians === null) {
    return (
      <div className="h-14 animate-pulse rounded-md bg-ivy/[0.03]" aria-busy="true" aria-label="Loading guardians" />
    )
  }

  if (guardians.length === 0) {
    return (
      <p className="rounded-[14px] border border-dashed border-ivy/30 px-5 py-6 text-[15px] leading-relaxed text-moss">
        No guardians yet. Without one, nothing above the daily limit can ever be sent. Your own wallet can&apos;t be a
        guardian.
      </p>
    )
  }

  return (
    <ul className="border-t border-rule">
      {guardians.map((g) => (
        <li key={g} className="flex items-center justify-between gap-4 border-b border-rule py-3">
          <AddressChip address={g} explorer />
          <span className="text-sm text-moss">Signs requests above the limit</span>
        </li>
      ))}
    </ul>
  )
}
