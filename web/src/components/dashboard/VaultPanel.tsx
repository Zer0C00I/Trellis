'use client'

import { useEffect, useState } from 'react'
import { BN } from '@coral-xyz/anchor'
import { Plus } from 'lucide-react'
import { AddressChip } from './AddressChip'
import { formatUsdc, formatUsdcAmount } from './format'

const U64_MAX = new BN('18446744073709551615')

function untilUtcMidnight(nowMs: number) {
  const next = Math.floor(nowMs / 86_400_000 + 1) * 86_400_000
  const mins = Math.max(0, Math.round((next - nowMs) / 60_000))
  const h = Math.floor(mins / 60)
  const m = mins % 60
  return h > 0 ? `${h} h ${m} min` : `${m} min`
}

export function VaultPanel({
  balance,
  dailyLimit,
  spentToday,
  isPool,
  vaultAddress,
  onDeposit,
}: {
  balance: BN
  dailyLimit: BN
  spentToday: BN
  isPool: boolean
  vaultAddress: string
  onDeposit: () => void
}) {
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 30_000)
    return () => clearInterval(t)
  }, [])

  const unlimited = dailyLimit.eq(U64_MAX)
  const capped = isPool && !unlimited && !dailyLimit.isZero()
  const pct = capped ? Math.min(100, spentToday.muln(10000).div(dailyLimit).toNumber() / 100) : 0
  const left = capped ? BN.max(dailyLimit.sub(spentToday), new BN(0)) : null

  return (
    <section aria-labelledby="vault-heading" className="flex flex-col rounded-[14px] bg-ivy p-6 text-lichen sm:p-8">
      <div className="flex items-center justify-between gap-4">
        <h2 id="vault-heading" className="text-[15px] font-medium text-lichen/75">
          Vault balance
        </h2>
        <AddressChip address={vaultAddress} explorer tone="dark" />
      </div>
      <p className="tabular mt-2 text-[44px] font-semibold leading-none tracking-[-0.03em] sm:text-[56px]">
        {formatUsdc(balance)}
      </p>
      <p className="mt-2 text-[15px] text-lichen/70">USDC on Solana Devnet</p>

      <div className="mt-8">
        {capped ? (
          <>
            <div className="flex items-baseline justify-between gap-4 text-[15px]">
              <span>Sent today</span>
              <span className="tabular">
                {formatUsdcAmount(spentToday)} of {formatUsdcAmount(dailyLimit)}
              </span>
            </div>
            <div
              className="mt-2 h-2.5 overflow-hidden rounded-full bg-lichen/15"
              role="meter"
              aria-label="Daily limit used"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={Math.round(pct)}
            >
              <div className="allowance-fill h-full rounded-full bg-bloom" style={{ width: `${pct}%` }} />
            </div>
            <p className="mt-2 text-sm text-lichen/70">
              {left && left.isZero()
                ? 'Limit reached. Anything more waits for a guardian.'
                : `${formatUsdcAmount(left!)} left.`}{' '}
              Resets in {untilUtcMidnight(now)}.
            </p>
          </>
        ) : (
          <p className="text-[15px] text-lichen/75">Personal vault: owner withdrawals have no daily cap.</p>
        )}
      </div>

      <div className="mt-8 border-t border-lichen/15 pt-6">
        <button
          id="open-deposit-button"
          type="button"
          onClick={onDeposit}
          className="inline-flex h-11 items-center gap-2 rounded-md bg-lichen px-5 text-[15px] font-medium text-ivy transition-colors hover:bg-white"
        >
          <Plus className="size-4" /> Deposit USDC
        </button>
      </div>
    </section>
  )
}
