import { SiteShell } from '@/components/site/site-shell'
import { RULES } from '@/lib/rules'

function Specimen({ children, label }: { children: React.ReactNode; label: string }) {
  return (
    <figure className="rounded-[14px] border border-rule bg-paper p-6 sm:p-8" aria-label={label}>
      {children}
    </figure>
  )
}

function Feature({
  title,
  body,
  points,
  specimen,
  flip = false,
}: {
  title: string
  body: string
  points: string[]
  specimen: React.ReactNode
  flip?: boolean
}) {
  return (
    <section className="border-t border-rule">
      <div className="mx-auto grid max-w-[1200px] items-center gap-10 px-4 py-16 sm:px-6 md:grid-cols-2 md:gap-16 md:py-24">
        <div className={flip ? 'md:order-2' : ''}>
          <h2 className="text-[34px] font-semibold leading-tight sm:text-[42px]">{title}</h2>
          <p className="mt-5 max-w-[34rem] text-lg leading-relaxed text-moss">{body}</p>
          <ul className="mt-6 space-y-2.5">
            {points.map((p) => (
              <li key={p} className="flex gap-3 text-[17px]">
                <span aria-hidden className="mt-[0.6em] size-2 shrink-0 rounded-full bg-wisteria" />
                {p}
              </li>
            ))}
          </ul>
        </div>
        <div className={flip ? 'md:order-1' : ''}>{specimen}</div>
      </div>
    </section>
  )
}

export default function FeaturesPage() {
  return (
    <SiteShell>
      <section className="mx-auto max-w-[1200px] px-4 pb-14 pt-12 sm:px-6 md:pb-20 md:pt-20">
        <h1 className="max-w-3xl text-[42px] font-semibold leading-[1.04] tracking-[-0.03em] sm:text-[56px]">
          Rules your family can rely on, kept by code instead of a firm
        </h1>
        <p className="mt-6 max-w-2xl text-lg leading-relaxed text-moss sm:text-[19px]">
          Trellis replaces the usual estate-planning paperwork with a Solana program. You set the rules once, and the
          program checks them on every transaction.
        </p>
      </section>

      <Feature
        title="Your heirs, written into the program"
        body={`Name up to ${RULES.maxBeneficiaries} people. The vault pays them and no one else. Every new name waits ${RULES.beneficiaryWaitHours} hours before it can receive anything. When ownership has to change hands, guardians open a ${RULES.ownerHandoverHours}-hour handover that you can cancel before it completes.`}
        points={[
          'Withdrawals only go to people on your list',
          `A ${RULES.beneficiaryWaitHours}-hour wait on every new name`,
          `Guardian-led handover of ownership, with a ${RULES.ownerHandoverHours}-hour wait`,
        ]}
        specimen={
          <Specimen label="A family member waiting out the 48-hour period">
            <p className="text-sm text-moss">Family</p>
            <div className="mt-3 flex items-center justify-between border-y border-rule py-4">
              <span className="text-[19px] font-medium">Deniz (grandson)</span>
              <span className="inline-flex items-center gap-2 text-[15px] text-amber">
                <svg viewBox="0 0 20 20" className="size-5 -rotate-90" aria-hidden>
                  <circle cx="10" cy="10" r="8" fill="none" stroke="currentColor" strokeOpacity="0.2" strokeWidth="3" />
                  <circle
                    cx="10"
                    cy="10"
                    r="8"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="3"
                    strokeDasharray="50.3"
                    strokeDashoffset="32.5"
                    strokeLinecap="round"
                  />
                </svg>
                Can receive in 31 hours
              </span>
            </div>
            <div className="flex items-center justify-between border-b border-rule py-4">
              <span className="text-[19px] font-medium">Ana (daughter)</span>
              <span className="inline-flex items-center gap-2 text-[15px] text-sap">
                <span aria-hidden className="size-2.5 rounded-full bg-sap" /> Can receive now
              </span>
            </div>
          </Specimen>
        }
      />

      <Feature
        flip
        title="No single key moves serious money"
        body={`Appoint up to ${RULES.maxGuardians} guardians: family, a lawyer, an accountant. The owner can never be one. A withdrawal above the daily limit waits on-chain until a guardian signs. Skipping the wait for a new name takes ${RULES.fastTrackSignatures} different guardians.`}
        points={[
          `Up to ${RULES.maxGuardians} guardians per vault`,
          'A guardian signs every withdrawal above the limit',
          `${RULES.fastTrackSignatures} signatures to skip the wait for a new name`,
        ]}
        specimen={
          <Specimen label="An over-limit withdrawal approved by one guardian">
            <div className="flex items-baseline justify-between">
              <p className="text-[19px] font-semibold">
                <span className="tabular">250 USDC</span> to Ana
              </p>
              <span className="rounded-full bg-sap px-2.5 py-0.5 text-sm font-medium text-white">Approved</span>
            </div>
            <p className="mt-1 text-sm text-moss">Above the daily limit, so it needed a guardian.</p>
            <ul className="mt-5 border-t border-rule text-[15px]">
              {[
                ['Ana, daughter', 'Signed', 'text-sap'],
                ['Family lawyer', 'Not needed', 'text-moss'],
                ['Accountant', 'Not needed', 'text-moss'],
                ['You, the owner', "Can't approve", 'text-moss'],
              ].map(([who, state, tone]) => (
                <li key={who} className="flex justify-between border-b border-rule py-3">
                  <span>{who}</span>
                  <span className={tone}>{state}</span>
                </li>
              ))}
            </ul>
          </Specimen>
        }
      />

      <Feature
        title="A hard ceiling on every day"
        body="Set a daily USDC limit. The Solana program enforces it, not a server, and resets it every UTC day. A withdrawal that would cross the line doesn't fail silently. It becomes a request for a guardian to approve."
        points={[
          'Caps how fast a vault can be drained',
          'Resets automatically at 00:00 UTC',
          'Anything above the limit goes to a guardian',
        ]}
        specimen={
          <Specimen label="A day's allowance, mostly used">
            <div className="rounded-[10px] bg-ivy p-6 text-lichen">
              <div className="flex items-baseline justify-between text-[15px]">
                <span>Sent today</span>
                <span className="tabular">85 USDC of {RULES.dailyLimitUsdc} USDC</span>
              </div>
              <div className="mt-2 h-2.5 overflow-hidden rounded-full bg-lichen/15">
                <div className="h-full w-[85%] rounded-full bg-bloom" />
              </div>
              <p className="mt-2 text-sm text-lichen/70">15 USDC left. Resets in 6 h 40 min.</p>
            </div>
            <p className="mt-5 border-l-4 border-amber bg-amber/[0.08] px-4 py-3 text-[15px]">
              Sending 40 USDC to Deniz is above today&apos;s limit, so it will wait for a guardian.
            </p>
          </Specimen>
        }
      />

      <Feature
        flip
        title="Fast, cheap, and always on"
        body="Trellis runs on Solana, so transfers settle in well under a second for a fraction of a cent. The vault account is rent-exempt, so it stays on-chain with no upkeep, and there's no Trellis server that has to be running."
        points={[
          'Settles in about 400 ms',
          'Fees of a fraction of a cent',
          'Rent paid once, when the vault is created',
        ]}
        specimen={
          <Specimen label="Settlement time">
            <p className="text-sm text-moss">Typical settlement</p>
            <p className="tabular mt-2 text-[66px] font-semibold leading-none tracking-[-0.04em]">
              400<span className="text-[27px] text-moss"> ms</span>
            </p>
            <p className="mt-4 inline-flex items-center gap-2 text-[15px] text-sap">
              <span aria-hidden className="size-2.5 rounded-full bg-sap" /> Finalized
            </p>
          </Specimen>
        }
      />
    </SiteShell>
  )
}
