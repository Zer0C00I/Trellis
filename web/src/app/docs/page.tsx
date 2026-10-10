import Link from 'next/link'
import { SiteHeader } from '@/components/site/site-header'
import { SiteFooter } from '@/components/site/site-footer'
import { CodeSnippet } from '@/components/site/code-snippet'

const NAV = [
  {
    group: 'The kiosk',
    items: [
      { label: 'Trellis Kiosk Terminal', href: '#kiosk' },
      { label: 'Family pairing', href: '#pairing' },
      { label: 'Same rules, on-chain', href: '#onchain-rules' },
    ],
  },
  {
    group: 'The program',
    items: [
      { label: 'Vault architecture', href: '#architecture' },
      { label: 'Guardian approvals', href: '#multisig' },
      { label: 'Daily spending limits', href: '#limits' },
    ],
  },
  {
    group: 'Plans',
    items: [
      { label: 'Pricing', href: '#pricing' },
      { label: 'Support', href: '#support' },
    ],
  },
]

function C({ children }: { children: React.ReactNode }) {
  return <code className="rounded-sm bg-ivy/[0.07] px-1.5 py-0.5 font-mono text-[0.88em]">{children}</code>
}

export default function DocsPage() {
  return (
    <div className="flex min-h-screen flex-col">
      <SiteHeader />
      <div className="mx-auto grid w-full max-w-[1200px] flex-1 gap-10 px-4 sm:px-6 md:grid-cols-[220px_1fr] md:gap-16">
        <aside className="md:sticky md:top-16 md:h-[calc(100vh-4rem)] md:overflow-y-auto md:py-12">
          <nav aria-label="Documentation" className="border-b border-rule py-6 md:border-0 md:py-0">
            {NAV.map((g) => (
              <div key={g.group} className="mb-7 last:mb-0">
                <h2 className="text-sm font-semibold text-ivy">{g.group}</h2>
                <ul className="mt-2 space-y-0.5 border-l border-rule">
                  {g.items.map((i) => (
                    <li key={i.href}>
                      <a
                        href={i.href}
                        className="-ml-px block border-l-2 border-transparent py-1.5 pl-3 text-[15px] text-moss transition-colors hover:border-wisteria hover:text-ivy"
                      >
                        {i.label}
                      </a>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </nav>
        </aside>

        <main id="main" className="min-w-0 max-w-[44rem] pb-24 md:py-12">
          <header id="introduction">
            <h1 className="text-[42px] font-semibold leading-[1.05] tracking-[-0.03em] sm:text-[53px]">
              Documentation
            </h1>
            <p className="mt-5 text-lg leading-relaxed text-moss">
              How the Trellis kiosk, the Solana program behind it, and the plans fit together.
            </p>
          </header>

          <section id="kiosk" className="mt-16 scroll-mt-24 border-t border-rule pt-12">
            <h2 className="text-[34px] font-semibold leading-tight">The Trellis Kiosk Terminal</h2>
            <p className="mt-5 text-[17px] leading-[1.7]">
              Browser wallets ask too much of the people who need protection most. The Trellis Kiosk Terminal is a
              touchscreen device for the vault holder: no browser, no seed phrase, no extensions. It does two things. It
              shows the balance, and it sends USDC to approved family.
            </p>
            <div className="mt-8 grid gap-8 sm:grid-cols-2">
              <div>
                <h3 className="text-[21px] font-semibold">One screen, one job</h3>
                <p className="mt-2 text-[16px] leading-relaxed text-moss">
                  Two wallets on screen: <em>My wallet</em> and <em>Family wallet</em>. Pick a person, pick an amount,
                  confirm. Large type, no jargon, nothing to configure after setup.
                </p>
              </div>
              <div id="pairing" className="scroll-mt-24">
                <h3 className="text-[21px] font-semibold">Family pairing</h3>
                <p className="mt-2 text-[16px] leading-relaxed text-moss">
                  On first boot the kiosk shows a QR code and a 6-digit fallback code. A relative scans it with the
                  Trellis app and links over Bluetooth. The kiosk only accepts the code currently on its own screen.
                </p>
              </div>
            </div>
            <div id="onchain-rules" className="mt-8 scroll-mt-24 border-l-4 border-wisteria bg-paper px-5 py-4">
              <h3 className="text-[19px] font-semibold">Same rules, on-chain</h3>
              <p className="mt-2 text-[16px] leading-relaxed">
                The kiosk gets no special powers. Every send goes through the Solana program: people on the list only,
                inside the daily USDC limit. Anything over the limit becomes a pending request until a guardian signs
                from their own device.
              </p>
            </div>
          </section>

          <section id="architecture" className="mt-16 scroll-mt-24 border-t border-rule pt-12">
            <h2 className="text-[34px] font-semibold leading-tight">The Solana program</h2>
            <p className="mt-5 text-[17px] leading-[1.7]">
              Every rule lives in one Anchor program on Solana. The kiosk, the family app, and this website are only
              interfaces. None of them can move funds the program would refuse.
            </p>
            <h3 className="mt-10 text-[21px] font-semibold">Vault account</h3>
            <div className="mt-4">
              <CodeSnippet />
            </div>

            <div id="multisig" className="mt-10 scroll-mt-24">
              <h3 className="text-[21px] font-semibold">Guardian approvals</h3>
              <p className="mt-2 text-[16px] leading-[1.7]">
                Each vault holds up to five guardian keys in a <C>TrustedApprovers</C> account. The owner can&apos;t be
                added. A withdrawal above the daily limit is stored as a <C>PendingWithdrawal</C> and only goes through
                when a listed guardian signs <C>approve_withdrawal</C> from their own wallet.{' '}
                <C>add_beneficiary_urgent</C> skips the 48-hour wait but needs two different guardian signatures.
              </p>
            </div>

            <div id="limits" className="mt-10 scroll-mt-24">
              <h3 className="text-[21px] font-semibold">Daily spending limits</h3>
              <p className="mt-2 text-[16px] leading-[1.7]">
                Limits aren&apos;t enforced by an API. The program checks them against{' '}
                <C>Clock::get()?.unix_timestamp</C>: it refuses a withdrawal if the amount plus <C>spent_today</C> would
                exceed <C>daily_limit</C>. The day index is <C>unix_timestamp / 86_400</C>. When it moves past{' '}
                <C>last_reset_day</C>, <C>spent_today</C> goes back to zero, so the limit resets at every UTC midnight.
              </p>
            </div>
          </section>

          <section id="pricing" className="mt-16 scroll-mt-24 border-t border-rule pt-12">
            <h2 className="text-[34px] font-semibold leading-tight">Pricing</h2>
            <p className="mt-5 text-[17px] leading-[1.7]">
              Personal vaults are free apart from Solana rent. Pro is $25 a month billed yearly, Advisor is $499 a year
              for firms, and the Terminal is a $12,500 device deposit plus a $9,600 yearly licence.
            </p>
            <Link
              href="/pricing"
              className="mt-4 inline-block font-medium text-wisteria underline decoration-wisteria/40 underline-offset-4 hover:decoration-wisteria"
            >
              Compare plans
            </Link>
          </section>

          <section id="support" className="mt-16 scroll-mt-24 border-t border-rule pt-12">
            <h2 className="text-[34px] font-semibold leading-tight">Support</h2>
            <p className="mt-5 text-[17px] leading-[1.7]">
              Email{' '}
              <a
                href="mailto:trellisvault@gmail.com"
                className="text-wisteria underline decoration-wisteria/40 underline-offset-4 hover:decoration-wisteria"
              >
                trellisvault@gmail.com
              </a>
              . Terminal customers get business-day support and next-business-day device replacement.
            </p>
          </section>
        </main>
      </div>
      <SiteFooter />
    </div>
  )
}
