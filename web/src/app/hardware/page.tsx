import Image from 'next/image'
import Link from 'next/link'

export default function HardwarePage() {
  return (
    <div className="min-h-screen bg-black font-sans selection:bg-emerald-500/30 selection:text-emerald-200">
      {/* Global Navigation */}
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
          <Link href="/features" className="hover:text-white transition-colors">
            Features
          </Link>
          <Link href="/hardware" className="text-white transition-colors">
            Kiosk
          </Link>
          <Link href="/pricing" className="hover:text-white transition-colors">
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

      {/* Header */}
      <section className="relative bg-black text-white pt-48 pb-16 px-8 overflow-hidden flex flex-col items-center text-center">
        <div className="absolute top-[-10%] left-1/2 -translate-x-1/2 w-[800px] h-[600px] bg-gradient-to-r from-emerald-900/40 via-green-900/20 to-black blur-[120px] pointer-events-none"></div>

        <div className="relative z-10 max-w-5xl mx-auto flex flex-col items-center">
          <h1 className="text-5xl md:text-7xl font-bold tracking-tight text-white mb-6 leading-tight">
            One screen.
            <br />
            One job.
          </h1>
          <p className="text-xl text-slate-400 max-w-2xl mx-auto font-light leading-relaxed mb-12">
            The Trellis Kiosk is a single-purpose app for an older person&apos;s own device. Large type, few choices,
            and no way to wander into a web page or a stranger&apos;s address.
          </p>
        </div>
      </section>

      {/* Hardware Mockup Hero */}
      <section className="bg-black pb-32 px-8 relative z-10">
        <div className="max-w-[1000px] mx-auto">
          <div className="w-full bg-[#050505] border border-white/10 p-2 md:p-6 rounded-[2rem] shadow-[0_0_80px_rgba(16,185,129,0.15)] relative overflow-hidden">
            <div className="absolute inset-0 bg-gradient-to-b from-emerald-500/10 to-transparent pointer-events-none"></div>
            {/* Tablet/Kiosk screen */}
            <div className="aspect-[16/10] bg-black border border-white/5 rounded-2xl overflow-hidden relative">
              <div className="absolute inset-0 flex flex-col">
                {/* Status Bar */}
                <div className="h-8 border-b border-white/10 flex justify-between items-center px-4 bg-white/5 text-[10px] font-mono text-slate-500">
                  <div>TRELLIS KIOSK</div>
                  <div className="flex gap-4">
                    <span className="text-emerald-500">DEVNET</span>
                    <span>EXAMPLE SCREEN</span>
                  </div>
                </div>
                {/* Main UI */}
                <div className="flex-grow flex flex-col items-center justify-center relative p-8 gap-8">
                  <div className="text-center">
                    <p className="text-slate-500 font-mono text-xs uppercase tracking-widest mb-2">Your balance</p>
                    <div className="text-white text-5xl font-medium tracking-wide">$120.00</div>
                  </div>
                  <div className="flex gap-4">
                    <div className="px-8 py-4 rounded-2xl bg-emerald-500/20 border border-emerald-500/40 text-emerald-300 text-xl">
                      Send to Anna
                    </div>
                    <div className="px-8 py-4 rounded-2xl bg-white/5 border border-white/10 text-slate-300 text-xl">
                      Send to Mark
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* What the kiosk is */}
      <section className="bg-white text-black py-32 px-8">
        <div className="max-w-[1200px] mx-auto">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-16 mb-24">
            <div>
              <span className="text-xs font-mono font-bold uppercase tracking-widest text-emerald-500 mb-6 block">
                WHY A KIOSK
              </span>
              <h2 className="text-4xl md:text-5xl font-bold tracking-tight mb-6">Take the dangerous parts away.</h2>
              <p className="text-lg text-slate-600 leading-relaxed font-medium mb-6">
                Most crypto scams against older people work through things a normal wallet allows: pasting an unknown
                address, approving a strange site, reading out a seed phrase.
                <br />
                <br />
                The kiosk simply does not have those things. It shows the balance and the people on the approved list,
                and that is all it does.
              </p>
            </div>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
              <div className="bg-slate-50 border border-slate-200 p-8 rounded-3xl">
                <h4 className="font-bold text-lg mb-2">One app only</h4>
                <p className="text-sm text-slate-600">
                  Built with Slint as the only application on the device. The planned device image (Alpine Linux with
                  the cage kiosk compositor) will run nothing else.
                </p>
              </div>
              <div className="bg-slate-50 border border-slate-200 p-8 rounded-3xl">
                <h4 className="font-bold text-lg mb-2">Closed address book</h4>
                <p className="text-sm text-slate-600">
                  Sending is limited to the vault&apos;s approved payees, and the on-chain program refuses anyone else
                  even if the screen were bypassed.
                </p>
              </div>
              <div className="bg-slate-50 border border-slate-200 p-8 rounded-3xl">
                <h4 className="font-bold text-lg mb-2">No seed phrase</h4>
                <p className="text-sm text-slate-600">
                  The device creates its own key on first run and keeps it in a local key file. In this prototype that
                  file is not encrypted and there is no secure chip or fingerprint reader, so physical access to the
                  device matters.
                </p>
              </div>
              <div className="bg-slate-50 border border-slate-200 p-8 rounded-3xl">
                <h4 className="font-bold text-lg mb-2">Pairs over Bluetooth</h4>
                <p className="text-sm text-slate-600">
                  During first-run setup the kiosk receives one family member&apos;s address over Bluetooth Low Energy
                  and registers them. This is the only time the kiosk itself adds a person.
                </p>
              </div>
            </div>
          </div>

          <div className="mt-16 pt-16 border-t border-slate-200 text-center">
            <h3 className="text-2xl font-bold mb-6">See how the rules work</h3>
            <Link
              href="/features"
              className="inline-block bg-emerald-500 hover:bg-emerald-600 text-white font-bold py-4 px-8 rounded-full transition-colors uppercase tracking-widest text-sm"
            >
              View Features
            </Link>
          </div>
        </div>
      </section>

      {/* Footer */}
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
                  <Link href="/features" className="hover:text-emerald-400 transition-colors">
                    Features
                  </Link>
                </li>
                <li>
                  <Link href="/hardware" className="hover:text-emerald-400 transition-colors">
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
