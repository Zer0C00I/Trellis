import { useState, useEffect } from 'react'
import { PublicKey } from '@solana/web3.js'
import { useAnchorProvider } from '../../config/useAnchorProvider'
import { AddressChip } from './AddressChip'
import { relativeHours } from './format'

export interface Beneficiary {
  publicKey: PublicKey
  name: string
  address: PublicKey
  activeAfter: number // Unix timestamp; payee can be paid from this moment
}

interface BeneficiaryTableProps {
  vaultPda: PublicKey
  refreshCounter: number
  onLoaded?: (beneficiaries: Beneficiary[]) => void
}

// Matches BENEFICIARY_COOLDOWN_SECS in programs/family-vault/src/state.rs; only used to draw the ring.
const COOLDOWN_SECS = 48 * 3600

function CooldownRing({ progress }: { progress: number }) {
  const r = 8
  const c = 2 * Math.PI * r
  return (
    <svg viewBox="0 0 20 20" className="size-5 -rotate-90" aria-hidden>
      <circle cx="10" cy="10" r={r} fill="none" stroke="currentColor" strokeOpacity="0.2" strokeWidth="3" />
      <circle
        cx="10"
        cy="10"
        r={r}
        fill="none"
        stroke="currentColor"
        strokeWidth="3"
        strokeDasharray={c}
        strokeDashoffset={c * (1 - progress)}
        strokeLinecap="round"
      />
    </svg>
  )
}

export function BeneficiaryTable({ vaultPda, refreshCounter, onLoaded }: BeneficiaryTableProps) {
  const { program } = useAnchorProvider()
  const [beneficiaries, setBeneficiaries] = useState<Beneficiary[]>([])
  const [loading, setLoading] = useState(true)
  // Captured when the list is fetched, so rendering stays pure.
  const [nowSecs, setNowSecs] = useState(0)

  useEffect(() => {
    if (!program) return
    let cancelled = false

    const fetchBeneficiaries = async () => {
      try {
        setLoading(true)
        // Only this vault's list (PDA: ["beneficiaries", vault]).
        const [listPda] = PublicKey.findProgramAddressSync(
          [Buffer.from('beneficiaries'), vaultPda.toBuffer()],
          program.programId,
        )
        const list = await program.account.beneficiaryList.fetchNullable(listPda)

        const result: Beneficiary[] = []
        if (list) {
          for (let i = 0; i < list.count; i++) {
            const entry = list.entries[i]
            const nameBytes = Uint8Array.from(entry.name)
            const nameStr = new TextDecoder().decode(nameBytes.slice(0, entry.nameLen))
            result.push({
              publicKey: entry.address,
              name: nameStr,
              address: entry.address,
              activeAfter: entry.activeAfter.toNumber(),
            })
          }
        }
        if (!cancelled) {
          setBeneficiaries(result)
          setNowSecs(Math.floor(Date.now() / 1000))
          onLoaded?.(result)
        }
      } catch (err) {
        console.error('Failed to fetch beneficiaries:', err)
      } finally {
        if (!cancelled) setLoading(false)
      }
    }

    fetchBeneficiaries()
    return () => {
      cancelled = true
    }
    // onLoaded is a notification only; re-fetching when its identity changes would loop.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [program, vaultPda, refreshCounter])

  // The program enforces the cooldown itself and stores the exact moment the
  // payee becomes usable, so show that rather than recomputing 48h client-side.
  const status = (activeAfter: number) => {
    const remainingSecs = activeAfter - nowSecs
    if (remainingSecs <= 0) {
      return (
        <span className="inline-flex items-center gap-2 text-[15px] text-sap">
          <span aria-hidden className="size-2.5 rounded-full bg-sap" />
          Can receive now
        </span>
      )
    }
    const progress = Math.min(1, Math.max(0, 1 - remainingSecs / COOLDOWN_SECS))
    return (
      <span className="inline-flex items-center gap-2 text-[15px] text-amber">
        <CooldownRing progress={progress} />
        Can receive {relativeHours(remainingSecs)}
      </span>
    )
  }

  if (loading && beneficiaries.length === 0) {
    return (
      <div className="space-y-px" aria-busy="true" aria-label="Loading family members">
        {[0, 1].map((i) => (
          <div key={i} className="h-16 animate-pulse border-b border-rule bg-ivy/[0.03]" />
        ))}
      </div>
    )
  }

  if (beneficiaries.length === 0) {
    return (
      <p className="rounded-[14px] border border-dashed border-ivy/30 px-5 py-6 text-[15px] leading-relaxed text-moss">
        Nobody can receive from this vault yet. Add a family member by their Solana address. They can be paid 48 hours
        after you add them.
      </p>
    )
  }

  return (
    <ul className="border-t border-rule">
      {beneficiaries.map((b) => (
        <li
          key={b.publicKey.toString()}
          className="grid grid-cols-1 gap-1 border-b border-rule py-4 sm:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)_minmax(0,1.2fr)] sm:items-center sm:gap-4"
        >
          <span className="truncate text-[17px] font-medium">{b.name}</span>
          <AddressChip address={b.address.toString()} />
          <span className="sm:text-right">{status(b.activeAfter)}</span>
        </li>
      ))}
    </ul>
  )
}
