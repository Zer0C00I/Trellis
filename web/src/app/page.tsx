import Image from 'next/image';
import Link from 'next/link';

export default function Home() {
  return (
    <div className="min-h-screen bg-black font-sans selection:bg-blue-500/30 selection:text-blue-200">
      
      {/* 1. Global Navigation */}
      <nav className="fixed top-6 left-1/2 -translate-x-1/2 w-[95%] max-w-6xl rounded-full bg-black/40 backdrop-blur-2xl border border-white/10 z-50 px-6 py-4 flex justify-between items-center shadow-[0_0_40px_-10px_rgba(0,0,0,0.5)]">
        <Link href="/" className="flex items-center gap-2">
          <Image src="/logo.png" alt="Trellis" width={240} height={80} className="h-20 w-auto -my-6 invert mix-blend-screen" />
        </Link>
        <div className="hidden md:flex items-center gap-8 text-sm font-medium text-gray-300">
          <Link href="/features" className="hover:text-white transition-colors">Features</Link>
          <Link href="/hardware" className="hover:text-white transition-colors">Hardware</Link>
          <Link href="/pricing" className="hover:text-white transition-colors">Pricing</Link>
          <Link href="/docs" className="hover:text-white transition-colors">Docs</Link>
        </div>
        <Link href="/dashboard">
          <button className="bg-white hover:bg-slate-200 text-black text-sm font-bold rounded-full px-6 py-2.5 transition-colors">
            Launch App
          </button>
        </Link>
      </nav>

      {/* 2. Hero Section (Web3 Glow) */}
      <section className="relative bg-black text-white pt-48 pb-32 px-8 overflow-hidden flex flex-col items-center text-center">
        <div className="absolute top-[-10%] left-1/2 -translate-x-1/2 w-[800px] h-[600px] bg-gradient-to-r from-blue-900/40 via-emerald-900/20 to-black blur-[120px] pointer-events-none"></div>
        
        <div className="relative z-10 max-w-5xl mx-auto flex flex-col items-center">
          <div className="text-emerald-400 font-mono text-sm tracking-widest uppercase mb-6">
            THE SELF-CUSTODIAL ESTATE PROTOCOL
          </div>
          
          <h1 className="text-6xl md:text-8xl font-bold tracking-tight text-white mb-6 leading-tight">
            Generational wealth.<br />
            Nothing else.
          </h1>
          
          <p className="text-xl text-slate-400 max-w-2xl mx-auto font-light leading-relaxed mb-12">
            No banks. No managers. Pure mathematical consensus. The only fiduciary-grade trust and estate protocol built natively for the Solana ecosystem.
          </p>
          
          <div className="flex flex-col sm:flex-row gap-4 w-full sm:w-auto justify-center">
            <Link href="/dashboard" className="w-full sm:w-auto">
              <button className="w-full sm:w-auto bg-blue-600 text-white hover:bg-blue-500 rounded-full px-8 py-4 font-medium transition-all hover:scale-105 shadow-[0_0_20px_-5px_rgba(37,99,235,0.5)]">
                Deploy Vault
              </button>
            </Link>
            <Link href="/docs" className="w-full sm:w-auto">
              <button className="w-full sm:w-auto bg-white/5 border border-white/10 text-white hover:bg-white/10 rounded-full px-8 py-4 font-medium backdrop-blur-sm transition-colors">
                Read the Docs
              </button>
            </Link>
          </div>
        </div>
      </section>

      {/* 3. Features Section (Comprehensive) */}
      <section id="features" className="bg-white text-black py-32 px-8">
        <div className="max-w-[1200px] mx-auto">
          <div className="text-center mb-20">
            <span className="text-xs font-mono font-bold uppercase tracking-widest text-emerald-500 mb-6 block">
              [ BLOCK: 849201A3F ] CORE CAPABILITIES
            </span>
            <h2 className="text-4xl md:text-6xl font-bold tracking-tight text-slate-900 leading-[1.1]">
              Uncompromising security for<br />your digital legacy.
            </h2>
          </div>
          
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-6">
            {/* Feature 1 */}
            <div className="bg-slate-50 border border-slate-200 rounded-3xl p-10 shadow-xl shadow-slate-200/50 flex flex-col justify-between">
              <div>
                <div className="w-12 h-12 bg-emerald-100 text-emerald-500 rounded-2xl flex items-center justify-center mb-8">
                  <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" /></svg>
                </div>
                <h3 className="text-blue-600 font-bold text-xl mb-4">No Identity Trail</h3>
                <p className="text-base text-slate-600 leading-relaxed font-medium">
                  Execute inheritance and withdrawal limits strictly on-chain without relying on centralized intermediaries or exposing any personal data.
                </p>
              </div>
            </div>
            
            {/* Feature 2 */}
            <div className="bg-slate-50 border border-slate-200 rounded-3xl p-10 shadow-xl shadow-slate-200/50 flex flex-col justify-between">
              <div>
                <div className="w-12 h-12 bg-blue-100 text-blue-500 rounded-2xl flex items-center justify-center mb-8">
                  <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" /></svg>
                </div>
                <h3 className="text-blue-600 font-bold text-xl mb-4">M-of-N Guardians</h3>
                <p className="text-base text-slate-600 leading-relaxed font-medium">
                  Distribute recovery shards among a trusted network of family, attorneys, and cold wallets. Ensure continuous access even if a single key is lost.
                </p>
              </div>
            </div>

            {/* Feature 3 */}
            <div className="bg-slate-50 border border-slate-200 rounded-3xl p-10 shadow-xl shadow-slate-200/50 flex flex-col justify-between">
              <div>
                <div className="w-12 h-12 bg-indigo-100 text-indigo-500 rounded-2xl flex items-center justify-center mb-8">
                  <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
                </div>
                <h3 className="text-blue-600 font-bold text-xl mb-4">Time-Locked Inheritance</h3>
                <p className="text-base text-slate-600 leading-relaxed font-medium">
                  Configure dead-man's switches and time-delayed asset unlocks to automate wealth transfer precisely when defined heartbeat conditions fail.
                </p>
              </div>
            </div>
          </div>
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            {/* Feature 4 */}
            <div className="bg-slate-50 border border-slate-200 rounded-3xl p-10 shadow-xl shadow-slate-200/50 flex flex-col justify-between">
              <div>
                <div className="w-12 h-12 bg-emerald-100 text-emerald-500 rounded-2xl flex items-center justify-center mb-8">
                  <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" /></svg>
                </div>
                <h3 className="text-blue-600 font-bold text-2xl mb-4">Built to Keep Working</h3>
                <p className="text-lg text-slate-600 leading-relaxed font-medium">
                  Smart contracts don't sleep. Trellis is a series of immutable Solana programs. There are no backend servers to fail, no subscription paywalls to freeze your funds, and nothing to crash. Your Vault's logic executes exactly as programmed, 100% of the time.
                </p>
              </div>
            </div>

            {/* Feature 5 */}
            <div className="bg-slate-50 border border-slate-200 rounded-3xl p-10 shadow-xl shadow-slate-200/50 flex flex-col justify-between">
              <div>
                <div className="w-12 h-12 bg-purple-100 text-purple-500 rounded-2xl flex items-center justify-center mb-8">
                  <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" /></svg>
                </div>
                <h3 className="text-blue-600 font-bold text-2xl mb-4">Audited & Verified</h3>
                <p className="text-lg text-slate-600 leading-relaxed font-medium">
                  Trellis core contracts are completely open-source and have been rigorously audited by top-tier Web3 security firms. We provide deterministic builds so anyone can verify that the on-chain byte code matches our public repositories.
                </p>
              </div>
            </div>
          </div>

        </div>
      </section>

      {/* 4. Hardware Kiosk Section (Black) */}
      <section id="hardware" className="bg-[#050505] text-white py-32 px-8 relative overflow-hidden">
        <div className="absolute right-0 top-1/2 -translate-y-1/2 w-[600px] h-[600px] bg-emerald-900/10 blur-[100px] pointer-events-none"></div>
        <div className="max-w-[1200px] mx-auto flex flex-col lg:flex-row justify-between items-center relative z-10 gap-16">
          <div className="max-w-xl z-20">
            <span className="text-xs font-mono font-bold uppercase tracking-widest text-emerald-500 mb-8 block">
              [ BLOCK: 910283B4C ] HARDWARE INTERFACE
            </span>
            <h2 className="text-4xl md:text-5xl font-bold tracking-tight leading-[1.1] mb-8">
              The Trellis Terminal Kiosk.
            </h2>
            <p className="text-lg text-slate-400 font-light leading-relaxed mb-8">
              We don't just rely on web browsers. The Trellis experience is anchored by our proprietary biometric Kiosk interface—a physical terminal designed for law firms, estate planners, and family offices to deploy and manage on-chain trusts with military-grade security.
            </p>
            <Link href="/docs#kiosk" className="inline-flex items-center gap-2 text-sm font-mono text-emerald-400 hover:text-emerald-300 transition-colors">
              Explore the Hardware Specs &rarr;
            </Link>
          </div>
          
          {/* Kiosk UI Mock */}
          <div className="z-10 w-full max-w-lg bg-black border border-white/10 p-6 rounded-2xl shadow-[0_0_50px_rgba(0,0,0,0.8)] relative overflow-hidden">
            <div className="absolute inset-0 bg-gradient-to-br from-emerald-500/5 to-transparent pointer-events-none"></div>
            <div className="flex justify-between items-center mb-8 border-b border-white/10 pb-4">
              <div className="font-bold text-white tracking-tight">Trellis<span className="text-emerald-500">OS</span></div>
              <div className="text-xs font-mono text-slate-500">TERMINAL: T-8492</div>
            </div>
            <div className="space-y-6">
              <div className="bg-white/5 border border-white/10 p-4 rounded-xl flex items-center justify-between">
                <div>
                  <div className="text-sm font-semibold text-white">Biometric Authentication</div>
                  <div className="text-xs text-slate-400 mt-1">Awaiting fingerprint scan...</div>
                </div>
                <div className="w-8 h-8 rounded-full border-2 border-emerald-500 border-t-transparent animate-spin"></div>
              </div>
              <div className="bg-white/5 border border-white/10 p-4 rounded-xl opacity-50">
                <div className="text-sm font-semibold text-white">NFC Keycard Sync</div>
                <div className="text-xs text-slate-400 mt-1">Insert Trellis Black Card</div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* 5. Network Rent / Pricing (Deep Blue) */}
      <section id="pricing" className="bg-blue-950 text-white py-32 px-8 relative overflow-hidden">
        {/* Subtle Green Radial Glow */}
        <div className="absolute top-0 right-0 w-[500px] h-[500px] bg-emerald-500/20 blur-[120px] pointer-events-none rounded-full translate-x-1/4 -translate-y-1/4"></div>
        <div className="max-w-[1200px] mx-auto relative z-10">
          <div className="text-center mb-20">
            <span className="text-xs font-mono font-bold uppercase tracking-widest text-emerald-400 mb-6 block">
              [ BLOCK: 593810F7A ] PRICING
            </span>
            <h2 className="text-4xl md:text-6xl font-bold tracking-tight leading-[1.1] mb-6">
              Start securing your legacy.
            </h2>
            <p className="text-xl md:text-2xl text-blue-200 font-light">
              Digital-first plans for individuals and growing firms.
            </p>
          </div>
          
          <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
            {/* Card 1: Free */}
            <div className="bg-black/40 backdrop-blur-md border border-blue-500/30 rounded-3xl p-8 hover:border-emerald-500/50 transition-colors w-full flex flex-col justify-between">
              <div>
                <h3 className="text-xl font-semibold tracking-tight mb-2 text-white">Trellis Personal</h3>
                <div className="text-3xl font-mono tracking-tight mb-6 text-emerald-400">Free</div>
                <p className="text-sm font-light text-blue-200 leading-relaxed mb-6">
                  Essential self-custody. Protect a personal digital estate.
                </p>
                <ul className="space-y-3 text-xs text-blue-200 mb-8">
                  <li className="flex items-center gap-2"><span className="text-emerald-400">✓</span> Web App Access</li>
                  <li className="flex items-center gap-2"><span className="text-emerald-400">✓</span> Max 2 Guardians</li>
                  <li className="flex items-center gap-2"><span className="text-emerald-400">✓</span> Pay only Solana network rent</li>
                </ul>
              </div>
              <div className="mt-4 pt-6 border-t border-blue-500/20 flex items-center justify-center">
                <Link href="/dashboard" className="text-sm font-bold text-white hover:text-emerald-400 transition-colors uppercase tracking-widest">Deploy Free &rarr;</Link>
              </div>
            </div>

            {/* Card 2: Pro */}
            <div className="bg-blue-900/20 backdrop-blur-md border border-blue-400/50 rounded-3xl p-8 shadow-[0_0_30px_rgba(59,130,246,0.1)] transition-colors w-full flex flex-col justify-between relative overflow-hidden">
              <div className="absolute top-4 right-4 px-3 py-1 bg-blue-500/20 text-blue-400 text-[10px] font-bold uppercase tracking-widest rounded-full">Recommended</div>
              <div>
                <h3 className="text-xl font-semibold tracking-tight mb-2 text-white">Trellis Pro</h3>
                <div className="text-3xl font-mono tracking-tight mb-2 text-blue-400">$25<span className="text-sm text-blue-400/60">/mo</span></div>
                <div className="text-xs text-blue-400/70 mb-6 font-mono">Billed annually ($300/yr)</div>
                <p className="text-sm font-light text-blue-200 leading-relaxed mb-6">
                  Advanced governance and alerts for active digital estates.
                </p>
                <ul className="space-y-3 text-xs text-blue-200 mb-8">
                  <li className="flex items-center gap-2"><span className="text-blue-400">✓</span> Up to 10 Guardians (M-of-N)</li>
                  <li className="flex items-center gap-2"><span className="text-blue-400">✓</span> SMS & Push Notification Alerts</li>
                  <li className="flex items-center gap-2"><span className="text-blue-400">✓</span> Downloadable Audit Logs</li>
                </ul>
              </div>
              <div className="mt-4 pt-6 border-t border-blue-500/20 flex items-center justify-center">
                <Link href="/dashboard" className="text-sm font-bold text-blue-400 hover:text-blue-300 transition-colors uppercase tracking-widest">Upgrade to Pro &rarr;</Link>
              </div>
            </div>

            {/* Card 3: Advisor */}
            <div className="bg-black/40 backdrop-blur-md border border-blue-500/30 rounded-3xl p-8 hover:border-emerald-500/50 transition-colors w-full flex flex-col justify-between relative overflow-hidden">
              <div className="absolute top-4 right-4 px-3 py-1 bg-emerald-500/10 text-emerald-400 text-[10px] font-bold uppercase tracking-widest rounded-full">Firms</div>
              <div>
                <h3 className="text-xl font-semibold tracking-tight mb-2 text-white">Trellis Advisor</h3>
                <div className="text-3xl font-mono tracking-tight mb-2 text-emerald-400">$499<span className="text-sm text-emerald-400/60">/yr</span></div>
                <div className="text-xs text-blue-400/70 mb-6 font-mono">Flat annual rate</div>
                <p className="text-sm font-light text-blue-200 leading-relaxed mb-6">
                  For estate planners and small law firms managing multiple digital estates.
                </p>
                <ul className="space-y-3 text-xs text-blue-200 mb-8">
                  <li className="flex items-center gap-2"><span className="text-emerald-400">✓</span> Up to 50 client estates</li>
                  <li className="flex items-center gap-2"><span className="text-emerald-400">✓</span> Co-branded Vault Interface</li>
                  <li className="flex items-center gap-2"><span className="text-emerald-400">✓</span> Priority API Access</li>
                </ul>
              </div>
              <div className="mt-4 pt-6 border-t border-blue-500/20 flex items-center justify-center">
                <a href="mailto:trellisvault@gmail.com" className="text-sm font-bold text-white hover:text-emerald-400 transition-colors uppercase tracking-widest">Contact Sales &rarr;</a>
              </div>
            </div>
          </div>

          <div className="mt-16 text-center">
            <Link href="/pricing" className="text-emerald-400 hover:text-emerald-300 font-mono text-sm uppercase tracking-widest border-b border-emerald-400/30 hover:border-emerald-400 transition-all pb-1">
              View all plans including Trellis Terminal &rarr;
            </Link>
          </div>
        </div>
      </section>

      {/* 6. FAQ (Crisp White) */}
      <section className="bg-white text-black py-32 px-8">
        <div className="max-w-[800px] mx-auto">
          <span className="text-xs font-mono font-bold uppercase tracking-widest text-blue-600 mb-8 block text-center">
            [ BLOCK: 204918C8B ] FAQ
          </span>
          <h2 className="text-4xl md:text-6xl font-bold tracking-tight mb-16 leading-[1.1] text-slate-900 text-center">
            The short answers.
          </h2>
          
          <div className="space-y-4">
            {[
              { q: "What happens if I lose my key?", a: "Your Guardians can initiate a recovery threshold signature to safely transfer ownership." },
              { q: "How are rent fees handled?", a: "The initial deployment fee is calculated to cover permanent rent-exemption on Solana." },
              { q: "Can I upgrade my vault later?", a: "Yes, you can allocate more space and upgrade to a Multi-Sig deployment at any time." },
              { q: "Is the contract audited?", a: "Yes, our core smart contracts have been thoroughly audited by top-tier Web3 security firms." },
            ].map((faq, i) => (
              <details key={i} className="group bg-slate-50 border border-slate-200 rounded-2xl shadow-sm">
                <summary className="flex justify-between items-center p-6 md:p-8 text-lg font-bold text-blue-600 cursor-pointer list-none hover:text-emerald-500 transition-colors">
                  {faq.q}
                  <span className="text-2xl leading-none font-light text-slate-400 group-open:rotate-45 transition-transform">+</span>
                </summary>
                <div className="px-6 md:px-8 pb-8 text-slate-600 font-medium leading-relaxed">
                  {faq.a}
                </div>
              </details>
            ))}
          </div>
        </div>
      </section>

      {/* Footer (Polished) */}
      <footer className="bg-black text-slate-500 py-20 px-8 border-t border-white/10">
        <div className="max-w-[1200px] mx-auto">
          <div className="grid grid-cols-1 md:grid-cols-4 gap-12 mb-16">
            <div className="col-span-1 md:col-span-2">
              <div className="flex items-center gap-2 mb-2">
                <Image src="/logo.png" alt="Trellis" width={400} height={128} className="h-40 w-auto -my-12 -ml-8 invert mix-blend-screen" />
              </div>
              <p className="text-slate-400 text-sm font-light leading-relaxed max-w-sm">
                Generational wealth, secured on-chain. The self-custodial trust and estate protocol built natively for the Solana ecosystem.
              </p>
            </div>
            
            <div>
              <h4 className="text-white font-semibold mb-6 text-sm">Product</h4>
              <ul className="space-y-4 text-sm font-medium">
                <li><Link href="/dashboard" className="hover:text-emerald-400 transition-colors">Launch App</Link></li>
                <li><Link href="/features" className="hover:text-emerald-400 transition-colors">Features</Link></li>
                <li><Link href="/hardware" className="hover:text-emerald-400 transition-colors">Hardware</Link></li>
                <li><Link href="/pricing" className="hover:text-emerald-400 transition-colors">Pricing</Link></li>
              </ul>
            </div>
            
            <div>
              <h4 className="text-white font-semibold mb-6 text-sm">Developers & Social</h4>
              <ul className="space-y-4 text-sm font-medium">
                <li><Link href="/docs" className="hover:text-emerald-400 transition-colors">Documentation</Link></li>
                <li><Link href="https://github.com/your-repo" className="hover:text-emerald-400 transition-colors">GitHub</Link></li>
                <li><Link href="https://x.com/TrellisVault" target="_blank" className="hover:text-emerald-400 transition-colors text-emerald-400/80">X (Twitter)</Link></li>
              </ul>
            </div>
          </div>
          
          <div className="pt-8 border-t border-white/10 flex flex-col md:flex-row justify-between items-center gap-4 text-xs font-mono">
            <div>&copy; {new Date().getFullYear()} TrellisVault. All rights reserved.</div>
            <div className="flex gap-6 uppercase tracking-widest font-bold">
              <Link href="#" className="hover:text-white transition-colors">Privacy</Link>
              <Link href="#" className="hover:text-white transition-colors">Terms</Link>
            </div>
          </div>
        </div>
      </footer>

    </div>
  );
}
