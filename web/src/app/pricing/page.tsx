import Link from 'next/link'
import { Check } from 'lucide-react'
import { SiteShell } from '@/components/site/site-shell'

const PLANS = [
  {
    name: 'Personal',
    price: 'Free',
    note: 'You pay Solana network rent only',
    who: 'Self-custody for one person’s savings.',
    cta: { label: 'Create a free vault', href: '/dashboard' },
    features: ['Web app', 'Up to 2 guardians', 'Standard waiting periods', 'Pay only Solana network rent'],
  },
  {
    name: 'Pro',
    price: '$25',
    unit: '/month',
    note: 'Billed yearly at $300',
    who: 'Governance and alerts for an active family estate.',
    cta: { label: 'Start with Pro', href: '/dashboard' },
    features: [
      'Everything in Personal',
      'Up to 5 guardians',
      'SMS and push alerts',
      'Downloadable audit logs',
      'Phantom and Backpack support',
    ],
    highlight: true,
  },
  {
    name: 'Advisor',
    price: '$499',
    unit: '/year',
    note: 'Flat yearly rate',
    who: 'Estate planners and small law firms managing several estates.',
    cta: { label: 'Talk to sales', href: 'mailto:trellisvault@gmail.com' },
    features: [
      'Up to 50 client estates',
      'Co-branded vault interface',
      'Priority API access',
      'A dedicated success manager',
      'PDF estate reports',
    ],
  },
]

const TERMINAL = [
  'One Trellis kiosk (touchscreen, pairs with family phones by code)',
  'Up to 5 guardians for approvals',
  'Software licence for one estate or trust',
  'Support on business days, 8am to 8pm EST',
  'Next-business-day device replacement',
  'API access for practice-management tools',
  'Guided setup, including key generation',
]

export default function PricingPage() {
  return (
    <SiteShell>
      <section className="mx-auto max-w-[1200px] px-4 pb-12 pt-12 sm:px-6 md:pt-20">
        <h1 className="max-w-3xl text-[42px] font-semibold leading-[1.04] tracking-[-0.03em] sm:text-[56px]">
          Pricing
        </h1>
        <p className="mt-6 max-w-2xl text-lg leading-relaxed text-moss sm:text-[19px]">
          From a free personal vault to a kiosk at a relative&apos;s home. Start free and upgrade when your family needs
          more.
        </p>
      </section>

      <section className="mx-auto max-w-[1200px] px-4 pb-16 sm:px-6 md:pb-24">
        <div className="grid gap-6 lg:grid-cols-3 lg:gap-0 lg:border-t-2 lg:border-ivy">
          {PLANS.map((p, i) => (
            <div
              key={p.name}
              className={`flex flex-col border-t-2 border-ivy pt-6 lg:border-t-0 lg:px-8 lg:pb-2 ${i === 0 ? 'lg:pl-0' : 'lg:border-l lg:border-l-rule'} ${
                p.highlight ? 'lg:bg-paper lg:pt-8' : 'lg:pt-8'
              }`}
            >
              <div className="flex items-center gap-3">
                <h2 className="text-[21px] font-semibold">{p.name}</h2>
                {p.highlight && (
                  <span className="rounded-full bg-bloom px-2.5 py-0.5 text-sm font-medium text-wisteria-ink">
                    Most families
                  </span>
                )}
              </div>
              <p className="mt-4 text-[42px] font-semibold leading-none tracking-tight">
                <span className="tabular">{p.price}</span>
                {p.unit && <span className="text-[17px] font-normal text-moss">{p.unit}</span>}
              </p>
              <p className="mt-2 text-sm text-moss">{p.note}</p>
              <p className="mt-4 text-[16px] leading-relaxed">{p.who}</p>
              <ul className="mt-6 flex-1 space-y-2.5">
                {p.features.map((f) => (
                  <li key={f} className="flex gap-2.5 text-[15px]">
                    <Check className="mt-0.5 size-4 shrink-0 text-sap" strokeWidth={2.5} aria-hidden />
                    {f}
                  </li>
                ))}
              </ul>
              <Link
                href={p.cta.href}
                className={`mt-8 inline-flex h-11 items-center justify-center rounded-md px-5 text-[15px] font-medium transition-colors ${
                  p.highlight
                    ? 'bg-wisteria text-white hover:bg-wisteria-ink'
                    : 'border border-ivy/35 text-ivy hover:border-ivy/60 hover:bg-ivy/[0.05]'
                }`}
              >
                {p.cta.label}
              </Link>
            </div>
          ))}
        </div>
      </section>

      <section className="bg-ivy text-lichen">
        <div className="mx-auto grid max-w-[1200px] gap-12 px-4 py-16 sm:px-6 md:grid-cols-2 md:py-24">
          <div>
            <h2 className="text-[34px] font-semibold leading-tight sm:text-[42px]">Trellis Terminal</h2>
            <p className="mt-4 max-w-md text-lg leading-relaxed text-lichen/80">
              A dedicated kiosk for the vault holder. No browser, no seed phrase, and every rule enforced on-chain.
            </p>
            <dl className="mt-8 grid max-w-md grid-cols-2 gap-6 border-t border-lichen/20 pt-6">
              <div>
                <dt className="text-sm text-lichen/70">Device deposit</dt>
                <dd className="tabular mt-1 text-[34px] font-semibold leading-none">$12,500</dd>
              </div>
              <div>
                <dt className="text-sm text-lichen/70">Yearly licence</dt>
                <dd className="tabular mt-1 text-[34px] font-semibold leading-none">$9,600</dd>
              </div>
            </dl>
            <a
              href="mailto:trellisvault@gmail.com"
              className="mt-8 inline-flex h-12 items-center rounded-md bg-lichen px-6 font-medium text-ivy transition-colors hover:bg-white"
            >
              Talk to sales
            </a>
          </div>
          <div>
            <h3 className="text-[19px] font-semibold">What&apos;s included</h3>
            <ul className="mt-4 border-t border-lichen/20">
              {TERMINAL.map((t) => (
                <li key={t} className="flex gap-3 border-b border-lichen/20 py-3.5 text-[16px]">
                  <Check className="mt-1 size-4 shrink-0 text-bloom" strokeWidth={2.5} aria-hidden />
                  {t}
                </li>
              ))}
            </ul>
          </div>
        </div>
      </section>
    </SiteShell>
  )
}
