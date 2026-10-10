import Link from 'next/link'
import { Wordmark } from './wordmark'

export function SiteFooter() {
  return (
    <footer className="bg-ivy text-lichen/75">
      <div className="mx-auto grid max-w-[1200px] gap-10 px-4 py-14 sm:px-6 md:grid-cols-[1.4fr_1fr_1fr]">
        <div>
          <Wordmark tone="lichen" className="-ml-1 h-16 w-auto" />
          <p className="mt-4 max-w-sm text-[15px] leading-relaxed">
            A self-custodial family vault on Solana. Devnet prototype, not audited. Use test funds only.
          </p>
        </div>
        <nav aria-label="Product">
          <h2 className="text-[15px] font-semibold text-lichen">Product</h2>
          <ul className="mt-4 space-y-2.5 text-[15px]">
            <li>
              <Link className="hover:text-lichen" href="/dashboard">
                Dashboard
              </Link>
            </li>
            <li>
              <Link className="hover:text-lichen" href="/features">
                Features
              </Link>
            </li>
            <li>
              <Link className="hover:text-lichen" href="/hardware">
                Kiosk
              </Link>
            </li>
            <li>
              <Link className="hover:text-lichen" href="/pricing">
                Pricing
              </Link>
            </li>
          </ul>
        </nav>
        <nav aria-label="More">
          <h2 className="text-[15px] font-semibold text-lichen">More</h2>
          <ul className="mt-4 space-y-2.5 text-[15px]">
            <li>
              <Link className="hover:text-lichen" href="/docs">
                Documentation
              </Link>
            </li>
            <li>
              <a
                className="hover:text-lichen"
                href="https://x.com/TrellisVault"
                target="_blank"
                rel="noopener noreferrer"
              >
                X (Twitter)
              </a>
            </li>
            <li>
              <a className="hover:text-lichen" href="mailto:trellisvault@gmail.com">
                trellisvault@gmail.com
              </a>
            </li>
          </ul>
        </nav>
      </div>
      <div className="border-t border-lichen/15">
        <div className="mx-auto flex max-w-[1200px] flex-col gap-2 px-4 py-5 text-sm sm:flex-row sm:justify-between sm:px-6">
          <span>&copy; {new Date().getFullYear()} Trellis</span>
          <span>Running on Solana Devnet</span>
        </div>
      </div>
    </footer>
  )
}
