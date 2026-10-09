import Image from 'next/image'
import Link from 'next/link'

export default function FeaturesPage() {
  return (
    <div className="min-h-screen bg-black font-sans selection:bg-blue-500/30 selection:text-blue-200">
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
          <Link href="/features" className="text-white transition-colors">
            Features
          </Link>
          <Link href="/hardware" className="hover:text-white transition-colors">
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
        <div className="absolute top-[-10%] left-1/2 -translate-x-1/2 w-[800px] h-[600px] bg-gradient-to-r from-blue-900/40 via-purple-900/20 to-black blur-[120px] pointer-events-none"></div>

        <div className="relative z-10 max-w-5xl mx-auto flex flex-col items-center">
          <h1 className="text-5xl md:text-7xl font-bold tracking-tight text-white mb-6 leading-tight">
            Rules the chain enforces.
            <br />
            Not a server of ours.
          </h1>
          <p className="text-xl text-slate-400 max-w-2xl mx-auto font-light leading-relaxed mb-12">
            Every limit on this page is checked by the Solana program on each transaction. If a rule says no, the
            transaction fails, whoever is asking.
          </p>
        </div>
      </section>

      {/* Core Protocol Features */}
      <section className="bg-white text-black py-32 px-8 relative z-10">
        <div className="max-w-[1200px] mx-auto">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-16 mb-32 items-center">
            <div>
              <span className="text-xs font-mono font-bold uppercase tracking-widest text-emerald-500 mb-6 block">
                PAYEES
              </span>
              <h2 className="text-4xl md:text-5xl font-bold tracking-tight mb-6">
                Only people the family has approved.
              </h2>
              <p className="text-lg text-slate-600 leading-relaxed font-medium mb-6">
                The kiosk can only send to addresses on the vault&apos;s address book. A newly added payee cannot be
                paid for 48 hours, so a scammer cannot talk someone into adding them and paying out in one sitting.
              </p>
              <ul className="space-y-4 text-slate-700 font-medium">
                <li className="flex items-center gap-3">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-500"></span> Up to 10 payees per vault
                </li>
                <li className="flex items-center gap-3">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-500"></span> 48-hour wait before a new payee can
                  be paid
                </li>
                <li className="flex items-center gap-3">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-500"></span> The program rejects any other
                  recipient
                </li>
              </ul>
            </div>
            <div className="bg-slate-50 border border-slate-200 p-8 rounded-3xl shadow-2xl relative overflow-hidden h-[400px] flex items-center justify-center">
              <div className="bg-white border border-emerald-500/20 shadow-xl rounded-2xl p-6 w-full max-w-sm relative z-10">
                <div className="text-xs font-mono text-slate-400 mb-4 uppercase">Address book (example)</div>
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-mono text-slate-800">Anna</span>
                    <span className="text-xs font-bold text-emerald-600">ACTIVE</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-mono text-slate-800">Mark</span>
                    <span className="text-xs font-bold text-amber-600">ACTIVE IN 31H</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-16 mb-32 items-center">
            <div className="order-2 md:order-1 bg-[#050505] p-8 rounded-3xl shadow-2xl relative overflow-hidden h-[400px] flex items-center justify-center">
              <div className="bg-black border border-blue-500/30 shadow-[0_0_30px_rgba(59,130,246,0.15)] rounded-2xl p-6 w-full max-w-sm relative z-10">
                <div className="text-xs font-mono text-slate-400 mb-4 uppercase">Urgent add (example)</div>
                <div className="space-y-2">
                  <div className="flex items-center gap-2">
                    <div className="w-2 h-2 rounded-full bg-blue-500"></div>
                    <span className="text-xs font-mono text-white">Guardian 1 signed</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <div className="w-2 h-2 rounded-full bg-blue-500"></div>
                    <span className="text-xs font-mono text-white">Guardian 2 signed</span>
                  </div>
                  <div className="mt-4 text-xs text-blue-400 font-bold px-2 py-1 bg-blue-500/20 rounded inline-block">
                    PAYEE ACTIVE NOW
                  </div>
                </div>
              </div>
            </div>
            <div className="order-1 md:order-2">
              <span className="text-xs font-mono font-bold uppercase tracking-widest text-blue-500 mb-6 block">
                GUARDIANS
              </span>
              <h2 className="text-4xl md:text-5xl font-bold tracking-tight mb-6">Trusted people who can step in.</h2>
              <p className="text-lg text-slate-600 leading-relaxed font-medium mb-6">
                A vault can name up to 5 guardians. One guardian can approve a large transfer. To add a payee without
                the 48-hour wait, two different guardians must sign the same transaction.
              </p>
              <ul className="space-y-4 text-slate-700 font-medium">
                <li className="flex items-center gap-3">
                  <span className="w-1.5 h-1.5 rounded-full bg-blue-500"></span> Up to 5 guardians per vault
                </li>
                <li className="flex items-center gap-3">
                  <span className="w-1.5 h-1.5 rounded-full bg-blue-500"></span> The owner cannot be their own guardian
                </li>
                <li className="flex items-center gap-3">
                  <span className="w-1.5 h-1.5 rounded-full bg-blue-500"></span> Adding a second guardian to a personal
                  vault needs an existing guardian to co-sign
                </li>
              </ul>
            </div>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-16 mb-32 items-center">
            <div>
              <span className="text-xs font-mono font-bold uppercase tracking-widest text-purple-500 mb-6 block">
                SPENDING
              </span>
              <h2 className="text-4xl md:text-5xl font-bold tracking-tight mb-6">
                Her money has no cap. The family pool does.
              </h2>
              <p className="text-lg text-slate-600 leading-relaxed font-medium mb-6">
                A personal vault has no spending limit; the controls are on who can be paid. A shared family pool has a
                daily limit. A transfer above it cannot go through directly. The owner files a request, and a guardian
                approves it.
              </p>
              <ul className="space-y-4 text-slate-700 font-medium">
                <li className="flex items-center gap-3">
                  <span className="w-1.5 h-1.5 rounded-full bg-purple-500"></span> No limit on a personal vault
                </li>
                <li className="flex items-center gap-3">
                  <span className="w-1.5 h-1.5 rounded-full bg-purple-500"></span> Daily limit on the shared pool
                </li>
                <li className="flex items-center gap-3">
                  <span className="w-1.5 h-1.5 rounded-full bg-purple-500"></span> A guardian approves anything above
                  the limit
                </li>
              </ul>
            </div>
            <div className="bg-slate-50 border border-slate-200 p-8 rounded-3xl shadow-2xl relative overflow-hidden h-[400px] flex items-center justify-center">
              <div className="bg-white border border-purple-500/20 shadow-xl rounded-2xl p-6 w-full max-w-sm relative z-10">
                <div className="text-xs font-mono text-slate-400 mb-4 uppercase">Family pool (example)</div>
                <div className="text-3xl font-mono text-purple-500 mb-2">
                  $80 <span className="text-lg text-slate-400">/ $100 today</span>
                </div>
                <div className="w-full bg-slate-100 h-2 rounded-full mb-4">
                  <div className="bg-purple-500 h-full rounded-full w-[80%]"></div>
                </div>
                <div className="text-xs text-amber-600 font-bold uppercase tracking-widest">
                  $50 transfer: waiting for a guardian
                </div>
              </div>
            </div>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-16 mb-32 items-center">
            <div className="order-2 md:order-1 bg-[#050505] p-8 rounded-3xl shadow-2xl relative overflow-hidden h-[400px] flex items-center justify-center">
              <div className="bg-black border border-amber-500/30 shadow-[0_0_30px_rgba(59,130,246,0.15)] rounded-2xl p-6 w-full max-w-sm relative z-10">
                <div className="text-xs font-mono text-slate-400 mb-4 uppercase">Key rotation (example)</div>
                <div className="text-3xl font-mono text-amber-400 mb-4">
                  71h <span className="text-lg text-slate-600">left</span>
                </div>
                <div className="text-xs text-slate-400 mb-3">
                  Requested by a guardian. Cancel at any time before it executes.
                </div>
                <div className="text-xs text-amber-400 font-bold px-2 py-1 bg-amber-500/20 rounded inline-block">
                  PENDING
                </div>
              </div>
            </div>
            <div className="order-1 md:order-2">
              <span className="text-xs font-mono font-bold uppercase tracking-widest text-amber-500 mb-6 block">
                RECOVERY
              </span>
              <h2 className="text-4xl md:text-5xl font-bold tracking-tight mb-6">Lose a key, keep the money.</h2>
              <p className="text-lg text-slate-600 leading-relaxed font-medium mb-6">
                If the owner&apos;s key is lost or compromised, a guardian can start a rotation to a new key. It takes
                effect only after 72 hours, and the owner or any guardian can cancel it in that time. Once the delay has
                passed, anyone can trigger it.
              </p>
              <ul className="space-y-4 text-slate-700 font-medium">
                <li className="flex items-center gap-3">
                  <span className="w-1.5 h-1.5 rounded-full bg-amber-500"></span> 72-hour delay before a new key takes
                  over
                </li>
                <li className="flex items-center gap-3">
                  <span className="w-1.5 h-1.5 rounded-full bg-amber-500"></span> The owner or any guardian can cancel
                </li>
                <li className="flex items-center gap-3">
                  <span className="w-1.5 h-1.5 rounded-full bg-amber-500"></span> A guardian cannot rotate ownership to
                  themselves
                </li>
              </ul>
            </div>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-16 items-center">
            <div>
              <span className="text-xs font-mono font-bold uppercase tracking-widest text-blue-500 mb-6 block">
                NO SEED PHRASE
              </span>
              <h2 className="text-4xl md:text-5xl font-bold tracking-tight mb-6">
                Nothing to write down, nothing to lose.
              </h2>
              <p className="text-lg text-slate-600 leading-relaxed font-medium mb-6">
                The kiosk creates its own key on the device and keeps it there, so there is no seed phrase to copy,
                photograph or be talked out of. (In this prototype the key file is not yet encrypted.) It shows the
                balance and a short list of payees, and it cannot open arbitrary apps or addresses.
              </p>
              <ul className="space-y-4 text-slate-700 font-medium">
                <li className="flex items-center gap-3">
                  <span className="w-1.5 h-1.5 rounded-full bg-blue-500"></span> Key created and kept on the device
                </li>
                <li className="flex items-center gap-3">
                  <span className="w-1.5 h-1.5 rounded-full bg-blue-500"></span> One app: balance and send
                </li>
                <li className="flex items-center gap-3">
                  <span className="w-1.5 h-1.5 rounded-full bg-blue-500"></span> No importing wallets, no connecting to
                  unknown sites
                </li>
              </ul>
            </div>
            <div className="bg-slate-50 border border-slate-200 p-8 rounded-3xl shadow-2xl relative overflow-hidden h-[400px] flex items-center justify-center">
              <div className="bg-white border border-blue-500/20 shadow-xl rounded-2xl p-6 w-full max-w-sm relative z-10">
                <div className="text-xs font-mono text-slate-400 mb-2 uppercase">Kiosk key</div>
                <div className="text-xl font-mono text-slate-900 mb-6">Stored on this device only</div>
                <div className="text-sm text-slate-500 font-medium bg-slate-100 py-2 rounded-lg text-center">
                  No seed phrase shown
                </div>
              </div>
            </div>
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
