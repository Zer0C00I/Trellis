'use client'

import { BN, ProgramAccount } from '@coral-xyz/anchor'
import { PublicKey } from '@solana/web3.js'
import { shortAddress } from './AddressChip'
import { formatUsdcAmount, relativeHours } from './format'

interface PendingAccount {
  amount: BN
  to: PublicKey
  requestedAt: BN
  approvedBy: PublicKey | null
}

/** Over-limit withdrawals waiting for a guardian. Read-only: guardians approve from their own device. */
export function PendingList({
  requests,
  names,
  nowSecs,
}: {
  requests: ProgramAccount[]
  names: Map<string, string>
  nowSecs: number
}) {
  return (
    <section aria-labelledby="pending-heading">
      <h2 id="pending-heading" className="flex items-center gap-2 text-[19px] font-semibold">
        Waiting for a guardian
        {requests.length > 0 && (
          <span className="tabular grid min-w-6 place-items-center rounded-full bg-amber px-1.5 text-[13px] font-semibold text-white">
            {requests.length}
          </span>
        )}
      </h2>
      {requests.length === 0 ? (
        <p className="mt-3 text-[15px] leading-relaxed text-moss">
          Nothing is waiting. When you send more than the daily limit, the request shows up here until a guardian signs
          it from their own device.
        </p>
      ) : (
        <ul className="mt-3 border-t border-rule">
          {requests.map((r) => {
            const a = r.account as unknown as PendingAccount
            const to = a.to.toBase58()
            return (
              <li key={r.publicKey.toBase58()} className="border-b border-rule py-3">
                <p className="text-[15px]">
                  <span className="tabular font-semibold">{formatUsdcAmount(a.amount)}</span> to{' '}
                  {names.get(to) ?? <span className="font-mono text-[13px]">{shortAddress(to)}</span>}
                </p>
                <p className="mt-0.5 text-sm text-moss">
                  Asked {relativeHours(a.requestedAt.toNumber() - nowSecs)}
                  {a.approvedBy ? ', approved and ready to send' : ''}
                </p>
              </li>
            )
          })}
        </ul>
      )}
    </section>
  )
}
