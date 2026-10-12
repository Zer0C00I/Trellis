import Link from 'next/link'
import { SiteShell } from '@/components/site/site-shell'
import { LatticeHero } from '@/components/site/lattice-hero'
import { KioskDevice } from '@/components/site/kiosk-device'
import { RULES } from '@/lib/rules'

const STEPS = [
  {
    title: 'Create your vault',
    body: 'Connect Phantom on Devnet and create the vault. The keys stay in your wallet. Trellis never holds your funds.',
  },
  {
    title: 'Name your family',
    body: `Add up to ${RULES.maxBeneficiaries} people by wallet address. Each new name waits ${RULES.beneficiaryWaitHours} hours before it can be paid, so a stolen key can't add a stranger and pay them in one go.`,
  },
  {
    title: 'Fund it with USDC',
    body: `Deposit from your wallet. Up to ${RULES.dailyLimitUsdc} USDC can leave per day, and the count resets at 00:00 UTC.`,
  },
  {
    title: 'Add a guardian',
    body: 'Pick someone you trust to sign anything above the daily limit. The owner can never be their own guardian.',
  },
]

const SCHEDULE = [
  { rule: 'People who can receive from one vault', value: `${RULES.maxBeneficiaries}` },
  { rule: 'Wait before a new name can be paid', value: `${RULES.beneficiaryWaitHours} hours` },
  { rule: 'Daily limit, reset at 00:00 UTC', value: `${RULES.dailyLimitUsdc} USDC` },
  { rule: 'Guardians per vault', value: `up to ${RULES.maxGuardians}` },
  { rule: 'Guardian signatures to skip the wait for a name', value: `${RULES.fastTrackSignatures}` },
  { rule: 'Wait before ownership passes to someone else', value: `${RULES.ownerHandoverHours} hours` },
]

const PLANS = [
  { name: 'Personal', price: 'Free', note: 'Network rent only', who: 'One person protecting their own savings' },
  { name: 'Pro', price: '$25/mo', note: 'Billed yearly, $300', who: 'Families who want alerts and audit logs' },
  { name: 'Advisor', price: '$499/yr', note: 'Flat rate', who: 'Estate planners with up to 50 clients' },
  { name: 'Terminal', price: '$12,500', note: 'Device, plus $9,600/yr', who: 'A dedicated kiosk for the vault holder' },
]

const FAQ = [
  {
    q: 'What happens if I lose my key?',
    a: 'Your guardians can start a recovery that hands ownership to a new key. It only completes after a 72-hour wait, so you can cancel it if you still have access.',
  },
  {
    q: 'What does it cost to run?',
    a: 'Creating a vault pays Solana rent once, about 0.011 SOL. After that you pay normal network fees per transaction.',
  },
  {
    q: 'Who can move the money?',
    a: 'Only you, and only to people on your list. Anything above the daily limit waits for a guardian to sign.',
  },
  {
    q: 'Is the contract audited?',
    a: 'Not yet. Trellis runs on Solana Devnet as an unaudited prototype. Use test funds only.',
  },
]

export default function Home() {
  return (
    <SiteShell>
      {/* Hero */}
      <section className="mx-auto grid max-w-[1200px] items-center gap-10 px-4 pb-16 pt-10 sm:px-6 md:grid-cols-[1fr_1.05fr] md:gap-6 md:pb-24 md:pt-16">
        <div className="max-w-[34rem]">
          <h1 className="text-[42px] font-semibold leading-[1.02] tracking-[-0.035em] sm:text-[56px] lg:text-[66px]">
            Secure crypto inheritance without middlemen.
          </h1>
          <p className="mt-6 max-w-[30rem] text-lg leading-relaxed text-moss sm:text-[19px]">
            Your USDC stays in a vault only you control. You choose who it goes to, how much can leave each day, and who
            has to sign off on more.
          </p>
          <div className="mt-8 flex flex-col gap-3 sm:flex-row">
            <Link
              href="/dashboard"
              className="inline-flex h-12 items-center justify-center rounded-md bg-wisteria px-6 text-base font-medium text-white transition-colors hover:bg-wisteria-ink"
            >
              Create a vault
            </Link>
            <Link
              href="#rules"
              className="inline-flex h-12 items-center justify-center rounded-md border border-ivy/35 px-6 text-base font-medium text-ivy transition-colors hover:border-ivy/60 hover:bg-ivy/[0.05]"
            >
              See the rules
            </Link>
          </div>
          <p className="mt-6 text-sm text-moss">Runs on Solana Devnet. Not audited yet, so use test funds.</p>
        </div>
        <LatticeHero />
      </section>

      {/* Steps: a real sequence, so it's numbered */}
      <section className="border-t border-rule bg-paper">
        <div className="mx-auto max-w-[1200px] px-4 py-16 sm:px-6 md:py-24">
          <h2 className="max-w-xl text-[34px] font-semibold leading-tight sm:text-[42px]">
            Setting up takes four steps
          </h2>
          <ol className="mt-12 grid gap-x-8 gap-y-10 sm:grid-cols-2 lg:grid-cols-4">
            {STEPS.map((s, i) => (
              <li key={s.title} className="relative border-t-2 border-ivy pt-5">
                <span className="tabular text-[42px] font-semibold leading-none text-wisteria">{i + 1}</span>
                <h3 className="mt-4 text-[21px] font-semibold">{s.title}</h3>
                <p className="mt-2 text-[16px] leading-relaxed text-moss">{s.body}</p>
              </li>
            ))}
          </ol>
        </div>
      </section>

      {/* The schedule of rules */}
      <section id="rules" className="scroll-mt-20 border-t border-rule">
        <div className="mx-auto grid max-w-[1200px] gap-10 px-4 py-16 sm:px-6 md:grid-cols-[0.8fr_1.2fr] md:py-24">
          <div>
            <h2 className="text-[34px] font-semibold leading-tight sm:text-[42px]">What the program enforces</h2>
            <p className="mt-5 max-w-md text-lg leading-relaxed text-moss">
              These aren&apos;t settings in an app. They&apos;re checks inside the Solana program, so nobody can skip
              them, including us.
            </p>
            <Link
              href="/docs#architecture"
              className="mt-6 inline-block font-medium text-wisteria underline decoration-wisteria/40 underline-offset-4 hover:decoration-wisteria"
            >
              Read the contract notes
            </Link>
          </div>
          <dl className="self-center">
            {SCHEDULE.map((row) => (
              <div
                key={row.rule}
                className="flex flex-col gap-1 border-b border-rule py-4 first:border-t sm:flex-row sm:items-baseline sm:gap-3"
              >
                <dt className="text-[17px] text-ivy">{row.rule}</dt>
                <span className="leader hidden sm:block" aria-hidden />
                <dd className="tabular text-[21px] font-semibold text-ivy sm:text-right">{row.value}</dd>
              </div>
            ))}
          </dl>
        </div>
      </section>

      {/* Kiosk */}
      <section className="bg-ivy text-lichen">
        <div className="mx-auto grid max-w-[1200px] items-center gap-12 px-4 py-16 sm:px-6 md:grid-cols-2 md:py-24">
          <div className="max-w-lg">
            <h2 className="text-[34px] font-semibold leading-tight sm:text-[42px]">
              A kiosk for the relative who&apos;d never touch a seed phrase
            </h2>
            <p className="mt-5 text-lg leading-relaxed text-lichen/80">
              A touchscreen at home with one job: show the balance and send to approved family. It pairs with a
              relative&apos;s phone through a code on screen, and every transfer follows the same limits and guardian
              rules as the vault.
            </p>
            <Link
              href="/hardware"
              className="mt-8 inline-flex h-12 items-center rounded-md border border-lichen/40 px-6 font-medium text-lichen transition-colors hover:border-lichen/70 hover:bg-lichen/[0.06]"
            >
              About the kiosk
            </Link>
          </div>
          <KioskDevice className="justify-self-center" />
        </div>
      </section>

      {/* Pricing */}
      <section className="border-b border-rule">
        <div className="mx-auto max-w-[1200px] px-4 py-16 sm:px-6 md:py-24">
          <div className="flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
            <h2 className="text-[34px] font-semibold leading-tight sm:text-[42px]">Pricing</h2>
            <Link
              href="/pricing"
              className="font-medium text-wisteria underline decoration-wisteria/40 underline-offset-4 hover:decoration-wisteria"
            >
              Compare every feature
            </Link>
          </div>
          <div className="mt-10 grid border-t-2 border-ivy sm:grid-cols-2 lg:grid-cols-4">
            {PLANS.map((p, i) => (
              <div
                key={p.name}
                className={`border-b border-rule py-6 sm:px-6 ${i % 2 === 0 ? 'sm:pl-0' : 'sm:border-l'} lg:border-b-0 ${i > 0 ? 'lg:border-l lg:pl-6' : 'lg:pl-0'}`}
              >
                <h3 className="text-[17px] font-semibold">{p.name}</h3>
                <p className="tabular mt-3 text-[34px] font-semibold leading-none tracking-tight">{p.price}</p>
                <p className="mt-2 text-sm text-moss">{p.note}</p>
                <p className="mt-4 text-[16px] leading-relaxed">{p.who}</p>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Questions */}
      <section>
        <div className="mx-auto grid max-w-[1200px] gap-10 px-4 py-16 sm:px-6 md:grid-cols-[0.8fr_1.2fr] md:py-24">
          <h2 className="text-[34px] font-semibold leading-tight sm:text-[42px]">Questions</h2>
          <div>
            {FAQ.map((f) => (
              <details key={f.q} className="group border-b border-rule first:border-t">
                <summary className="flex cursor-pointer list-none items-center justify-between gap-6 py-5 text-[19px] font-medium [&::-webkit-details-marker]:hidden">
                  {f.q}
                  <span
                    aria-hidden
                    className="grid size-8 shrink-0 place-items-center rounded-full border border-rule text-moss transition-transform group-open:rotate-45"
                  >
                    +
                  </span>
                </summary>
                <p className="max-w-[60ch] pb-6 text-[17px] leading-relaxed text-moss">{f.a}</p>
              </details>
            ))}
          </div>
        </div>
      </section>
    </SiteShell>
  )
}
