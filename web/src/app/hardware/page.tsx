import Link from 'next/link'
import { SiteShell } from '@/components/site/site-shell'
import { KioskDevice } from '@/components/site/kiosk-device'
import { RULES } from '@/lib/rules'

const POINTS = [
  {
    title: 'Pays only your family',
    body: `The kiosk can only send to people on the vault's list. New names wait ${RULES.beneficiaryWaitHours} hours before they can receive anything.`,
  },
  {
    title: 'A daily limit it can’t exceed',
    body: 'Sends stay under the daily USDC limit. The program enforces it and resets it every UTC day.',
  },
  {
    title: 'Guardians for anything bigger',
    body: 'Anything over the limit waits on-chain until a guardian signs from their own device.',
  },
  {
    title: 'Pairs by QR code',
    body: 'A relative links their phone by scanning the code on screen. The kiosk accepts only the code it is showing.',
  },
]

export default function HardwarePage() {
  return (
    <SiteShell>
      <section className="bg-ivy text-lichen">
        <div className="mx-auto grid max-w-[1200px] items-center gap-12 px-4 py-14 sm:px-6 md:grid-cols-[1fr_1.1fr] md:py-24">
          <div>
            <h1 className="text-[42px] font-semibold leading-[1.04] tracking-[-0.03em] sm:text-[56px]">
              No browser. No seed phrase. Just family.
            </h1>
            <p className="mt-6 max-w-[32rem] text-lg leading-relaxed text-lichen/80">
              The Trellis Terminal is a touchscreen kiosk for the vault holder. It shows the balance and sends USDC to
              approved family. Nothing else.
            </p>
          </div>
          <KioskDevice className="justify-self-center" />
        </div>
      </section>

      <section>
        <div className="mx-auto grid max-w-[1200px] gap-12 px-4 py-16 sm:px-6 md:grid-cols-[0.9fr_1.1fr] md:py-24">
          <div>
            <h2 className="text-[34px] font-semibold leading-tight sm:text-[42px]">Why a kiosk?</h2>
            <p className="mt-5 max-w-md text-lg leading-relaxed text-moss">
              Browser wallets hand the most vulnerable person the most complicated tool: extensions, seed phrases, and
              phishing links one tap away.
            </p>
            <p className="mt-4 max-w-md text-lg leading-relaxed text-moss">
              The kiosk takes all of that away. Large type, a few screens, one action: send to family. The Solana
              program enforces every rule underneath it.
            </p>
          </div>
          <dl className="grid gap-x-10 sm:grid-cols-2">
            {POINTS.map((p) => (
              <div key={p.title} className="border-t-2 border-ivy py-5">
                <dt className="text-[19px] font-semibold">{p.title}</dt>
                <dd className="mt-2 text-[16px] leading-relaxed text-moss">{p.body}</dd>
              </div>
            ))}
          </dl>
        </div>
      </section>

      <section className="border-t border-rule bg-paper">
        <div className="mx-auto flex max-w-[1200px] flex-col items-start justify-between gap-6 px-4 py-14 sm:px-6 md:flex-row md:items-center">
          <div>
            <h2 className="text-[27px] font-semibold">Setting one up for a relative?</h2>
            <p className="mt-2 text-[17px] text-moss">The Terminal plan includes the device, setup, and support.</p>
          </div>
          <Link
            href="/pricing"
            className="inline-flex h-12 items-center rounded-md bg-ivy px-6 font-medium text-lichen transition-colors hover:bg-ivy-2"
          >
            See Terminal pricing
          </Link>
        </div>
      </section>
    </SiteShell>
  )
}
