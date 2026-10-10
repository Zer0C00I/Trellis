import Image from 'next/image'
import Link from 'next/link'

export default function PricingPage() {
  return (
    <div className="min-h-screen bg-black font-sans selection:bg-blue-500/30 selection:text-blue-200">
      <nav className="fixed top-6 left-1/2 -translate-x-1/2 w-[95%] max-w-6xl rounded-full bg-black/40 backdrop-blur-2xl border border-white/10 z-50 px-6 py-4 flex justify-between items-center shadow-[0_0_40px_-10px_rgba(0,0,0,0.5)]">
        <Link href="/" className="flex items-center gap-2">
          <Image
            src="/logo.png"
            alt="Trellis"
            width={240}
            height={80}
            className="h-20 w-auto -my-6 invert mix-blend-screen"
          />
        </Link>
        <div className="hidden md:flex items-center gap-8 text-sm font-medium text-gray-300">
          <Link href="/#features" className="hover:text-white transition-colors">
            Features
          </Link>
          <Link href="/#hardware" className="hover:text-white transition-colors">
            Kiosk
          </Link>
          <Link href="/pricing" className="text-white transition-colors">
            Pricing
          </Link>
          <Link href="/docs" className="hover:text-white transition-colors">
            Docs
          </Link>
        </div>
        <Link href="/dashboard">
          <button className="bg-white hover:bg-slate-200 text-black text-sm font-bold rounded-full px-6 py-2.5 transition-colors">
            Launch App
          </button>
        </Link>
      </nav>

      <section className="relative bg-black text-white pt-48 pb-16 px-8 overflow-hidden flex flex-col items-center text-center">
        <div className="absolute top-[-10%] left-1/2 -translate-x-1/2 w-[800px] h-[600px] bg-gradient-to-r from-emerald-900/40 via-blue-900/20 to-black blur-[120px] pointer-events-none"></div>

        <div className="relative z-10 max-w-5xl mx-auto flex flex-col items-center">
          <h1 className="text-5xl md:text-7xl font-bold tracking-tight text-white mb-6 leading-tight">
            Free for the people
            <br />
            who use it.
          </h1>
          <p className="text-xl text-slate-400 max-w-2xl mx-auto font-light leading-relaxed mb-12">
            The core of Trellis is free, always. Paid extras are an idea for later, and nothing is for sale yet.
          </p>
        </div>
      </section>

      <section className="bg-black text-white pb-32 px-8 relative z-10">
        <div className="max-w-[900px] mx-auto space-y-8">
          <div className="bg-black border border-white/10 rounded-3xl p-8 md:p-10">
            <h3 className="text-xl font-semibold tracking-tight mb-2 text-white">The core, free for the owner</h3>
            <div className="text-3xl font-mono tracking-tight mb-6 text-emerald-400">Free</div>
            <ul className="space-y-3 text-sm text-slate-300">
              <li className="flex items-start gap-3">
                <span className="text-emerald-400">✓</span> Seeing the balance and sending to approved payees
              </li>
              <li className="flex items-start gap-3">
                <span className="text-emerald-400">✓</span> The shared family pool and its daily limit
              </li>
              <li className="flex items-start gap-3">
                <span className="text-emerald-400">✓</span> Guardian approvals and key recovery
              </li>
            </ul>
            <p className="text-sm text-slate-500 leading-relaxed mt-6">
              Nobody&apos;s access to their own money depends on a subscription. The only cost today is Solana network
              rent and fees, paid in Devnet SOL while this is a prototype.
            </p>
          </div>

          <div className="bg-black border border-white/10 rounded-3xl p-8 md:p-10">
            <h3 className="text-xl font-semibold tracking-tight mb-2 text-white">
              Possible paid extras, not built yet
            </h3>
            <div className="text-sm font-mono tracking-tight mb-6 text-blue-400">
              PAID BY A RELATIVE, NEVER BY THE OWNER
            </div>
            <ul className="space-y-3 text-sm text-slate-300">
              <li className="flex items-start gap-3">
                <span className="text-blue-400">•</span> Spending analytics
              </li>
              <li className="flex items-start gap-3">
                <span className="text-blue-400">•</span> Notifications for the family
              </li>
              <li className="flex items-start gap-3">
                <span className="text-blue-400">•</span> Remote management of limits
              </li>
            </ul>
            <p className="text-sm text-slate-500 leading-relaxed mt-6">
              These are ideas, with no prices and no launch date. If they ever exist, they will not lock the owner out
              of the core.
            </p>
          </div>
        </div>
      </section>

      <footer className="bg-black text-slate-500 py-20 px-8 border-t border-white/10">
        <div className="max-w-[1200px] mx-auto">
          <div className="grid grid-cols-1 md:grid-cols-4 gap-12 mb-16">
            <div className="col-span-1 md:col-span-2">
              <div className="flex items-center gap-2 mb-2">
                <Image
                  src="/logo.png"
                  alt="Trellis"
                  width={400}
                  height={128}
                  className="h-40 w-auto -my-12 -ml-8 invert mix-blend-screen"
                />
              </div>
              <p className="text-slate-400 text-sm font-light leading-relaxed max-w-sm">
                A family vault on Solana: an older person&apos;s kiosk, a family app for oversight, and an on-chain
                program that enforces the rules. Prototype on Devnet.
              </p>
            </div>

            <div>
              <h4 className="text-white font-semibold mb-6 text-sm">Product</h4>
              <ul className="space-y-4 text-sm font-medium">
                <li>
                  <Link href="/dashboard" className="hover:text-emerald-400 transition-colors">
                    Launch App
                  </Link>
                </li>
                <li>
                  <Link href="/#features" className="hover:text-emerald-400 transition-colors">
                    Features
                  </Link>
                </li>
                <li>
                  <Link href="/#hardware" className="hover:text-emerald-400 transition-colors">
                    Kiosk
                  </Link>
                </li>
                <li>
                  <Link href="/pricing" className="hover:text-emerald-400 transition-colors">
                    Pricing
                  </Link>
                </li>
              </ul>
            </div>

            <div>
              <h4 className="text-white font-semibold mb-6 text-sm">Docs & Social</h4>
              <ul className="space-y-4 text-sm font-medium">
                <li>
                  <Link href="/docs" className="hover:text-emerald-400 transition-colors">
                    Documentation
                  </Link>
                </li>
                <li>
                  <Link
                    href="https://x.com/TrellisVault"
                    target="_blank"
                    className="hover:text-emerald-400 transition-colors"
                  >
                    X (Twitter)
                  </Link>
                </li>
                <li>
                  <a href="mailto:trellisvault@gmail.com" className="hover:text-emerald-400 transition-colors">
                    Contact
                  </a>
                </li>
              </ul>
            </div>
          </div>

          <div className="pt-8 border-t border-white/10 flex flex-col md:flex-row justify-between items-center gap-4 text-xs font-mono">
            <div>&copy; {new Date().getFullYear()} Trellis</div>
          </div>
        </div>
      </footer>
    </div>
  )
}
