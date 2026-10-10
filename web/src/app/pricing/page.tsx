import Image from 'next/image';
import Link from 'next/link';

export default function PricingPage() {
  return (
    <div className="min-h-screen bg-black font-sans selection:bg-blue-500/30 selection:text-blue-200">
      
      {/* Global Navigation */}
      <nav className="fixed top-6 left-1/2 -translate-x-1/2 w-[95%] max-w-6xl rounded-full bg-black/40 backdrop-blur-2xl border border-white/10 z-50 px-6 py-4 flex justify-between items-center shadow-[0_0_40px_-10px_rgba(0,0,0,0.5)]">
        <Link href="/" className="flex items-center gap-2">
          <Image src="/logo.png" alt="Trellis" width={240} height={80} className="h-20 w-auto -my-6 invert mix-blend-screen" />
        </Link>
        <div className="hidden md:flex items-center gap-8 text-sm font-medium text-gray-300">
          <Link href="/#features" className="hover:text-white transition-colors">Features</Link>
          <Link href="/#hardware" className="hover:text-white transition-colors">Hardware</Link>
          <Link href="/pricing" className="text-white transition-colors">Pricing</Link>
          <Link href="/docs" className="hover:text-white transition-colors">Docs</Link>
        </div>
        <Link href="/dashboard">
          <button className="bg-white hover:bg-slate-200 text-black text-sm font-bold rounded-full px-6 py-2.5 transition-colors">
            Launch App
          </button>
        </Link>
      </nav>

      {/* Pricing Header */}
      <section className="relative bg-black text-white pt-48 pb-16 px-8 overflow-hidden flex flex-col items-center text-center">
        <div className="absolute top-[-10%] left-1/2 -translate-x-1/2 w-[800px] h-[600px] bg-gradient-to-r from-emerald-900/40 via-blue-900/20 to-black blur-[120px] pointer-events-none"></div>
        
        <div className="relative z-10 max-w-5xl mx-auto flex flex-col items-center">
          <h1 className="text-5xl md:text-7xl font-bold tracking-tight text-white mb-6 leading-tight">
            Transparent pricing for<br />generational security.
          </h1>
          <p className="text-xl text-slate-400 max-w-2xl mx-auto font-light leading-relaxed mb-12">
            From free personal vaults to military-grade institutional hardware. Start today and upgrade as your digital estate grows.
          </p>
        </div>
      </section>

      {/* Pricing Grid */}
      <section className="bg-black text-white pb-32 px-8 relative z-10">
        <div className="max-w-[1200px] mx-auto">
          
          <h3 className="text-xl font-bold text-white mb-6 border-b border-white/10 pb-4">Digital Plans</h3>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-8 mb-24">
            
            {/* Free */}
            <div className="bg-black border border-white/10 rounded-3xl p-8 hover:border-emerald-500/50 transition-colors w-full flex flex-col justify-between">
              <div>
                <h3 className="text-xl font-semibold tracking-tight mb-2 text-white">Trellis Personal</h3>
                <div className="text-3xl font-mono tracking-tight mb-6 text-emerald-400">Free</div>
                <p className="text-sm font-light text-slate-400 leading-relaxed mb-6">
                  Essential self-custody. Protect a personal digital estate.
                </p>
                <ul className="space-y-3 text-sm text-slate-300 mb-8">
                  <li className="flex items-center gap-2"><span className="text-emerald-400">✓</span> Web App Access</li>
                  <li className="flex items-center gap-2"><span className="text-emerald-400">✓</span> Max 2 Guardians</li>
                  <li className="flex items-center gap-2"><span className="text-emerald-400">✓</span> Standard Time-locks</li>
                  <li className="flex items-center gap-2"><span className="text-emerald-400">✓</span> Pay only Solana network rent</li>
                </ul>
              </div>
              <div className="mt-4 pt-6 border-t border-white/10">
                <Link href="/dashboard" className="block w-full text-center bg-white/10 hover:bg-white/20 text-white font-bold py-3 rounded-full transition-colors uppercase tracking-widest text-xs">
                  Deploy Free
                </Link>
              </div>
            </div>

            {/* Pro */}
            <div className="bg-blue-900/10 backdrop-blur-md border border-blue-500/30 rounded-3xl p-8 shadow-[0_0_30px_rgba(59,130,246,0.1)] transition-colors w-full flex flex-col justify-between relative overflow-hidden">
              <div className="absolute top-4 right-4 px-3 py-1 bg-blue-500/20 text-blue-400 text-[10px] font-bold uppercase tracking-widest rounded-full">Recommended</div>
              <div>
                <h3 className="text-xl font-semibold tracking-tight mb-2 text-white">Trellis Pro</h3>
                <div className="text-3xl font-mono tracking-tight mb-2 text-blue-400">$25<span className="text-sm text-blue-400/60">/mo</span></div>
                <div className="text-xs text-blue-400/70 mb-6 font-mono">Billed annually ($300/yr)</div>
                <p className="text-sm font-light text-blue-200 leading-relaxed mb-6">
                  Advanced governance and alerts for active digital estates.
                </p>
                <ul className="space-y-3 text-sm text-blue-200 mb-8">
                  <li className="flex items-center gap-2"><span className="text-blue-400">✓</span> Everything in Free</li>
                  <li className="flex items-center gap-2"><span className="text-blue-400">✓</span> Up to 10 Guardians (M-of-N)</li>
                  <li className="flex items-center gap-2"><span className="text-blue-400">✓</span> SMS & Push Notification Alerts</li>
                  <li className="flex items-center gap-2"><span className="text-blue-400">✓</span> Downloadable Audit Logs</li>
                  <li className="flex items-center gap-2"><span className="text-blue-400">✓</span> Phantom & Backpack Integration</li>
                </ul>
              </div>
              <div className="mt-4 pt-6 border-t border-blue-500/20">
                <Link href="/dashboard" className="block w-full text-center bg-blue-600 hover:bg-blue-500 text-white font-bold py-3 rounded-full transition-colors uppercase tracking-widest text-xs">
                  Upgrade to Pro
                </Link>
              </div>
            </div>

            {/* Advisor */}
            <div className="bg-[#050505] border border-emerald-500/30 rounded-3xl p-8 hover:border-emerald-500/50 transition-colors w-full flex flex-col justify-between relative overflow-hidden">
              <div className="absolute top-4 right-4 px-3 py-1 bg-emerald-500/10 text-emerald-400 text-[10px] font-bold uppercase tracking-widest rounded-full">Firms</div>
              <div>
                <h3 className="text-xl font-semibold tracking-tight mb-2 text-white">Trellis Advisor</h3>
                <div className="text-3xl font-mono tracking-tight mb-2 text-emerald-400">$499<span className="text-sm text-emerald-400/60">/yr</span></div>
                <div className="text-xs text-emerald-400/70 mb-6 font-mono">Flat annual rate</div>
                <p className="text-sm font-light text-slate-300 leading-relaxed mb-6">
                  For estate planners and small law firms managing multiple digital estates.
                </p>
                <ul className="space-y-3 text-sm text-slate-300 mb-8">
                  <li className="flex items-center gap-2"><span className="text-emerald-400">✓</span> Manage up to 50 client estates</li>
                  <li className="flex items-center gap-2"><span className="text-emerald-400">✓</span> Co-branded Vault Interface</li>
                  <li className="flex items-center gap-2"><span className="text-emerald-400">✓</span> Priority API Access</li>
                  <li className="flex items-center gap-2"><span className="text-emerald-400">✓</span> Dedicated Success Manager</li>
                  <li className="flex items-center gap-2"><span className="text-emerald-400">✓</span> PDF Estate Reports</li>
                </ul>
              </div>
              <div className="mt-4 pt-6 border-t border-white/10">
                <a href="mailto:trellisvault@gmail.com" className="block w-full text-center bg-white text-black font-bold py-3 rounded-full transition-colors uppercase tracking-widest text-xs">
                  Contact Sales
                </a>
              </div>
            </div>

          </div>

          {/* Enterprise Plan */}
          <h3 className="text-xl font-bold text-emerald-400 mb-6 border-b border-emerald-500/20 pb-4">Institutional Hardware</h3>
          <div className="bg-black/60 backdrop-blur-md border-2 border-emerald-500/50 rounded-3xl p-10 lg:p-12 shadow-[0_0_40px_rgba(16,185,129,0.15)] transition-colors w-full flex flex-col lg:flex-row gap-12 overflow-hidden">
            <div className="lg:w-1/3 flex flex-col justify-between">
              <div>
                <div className="inline-block px-3 py-1 bg-emerald-500/20 text-emerald-400 text-xs font-bold uppercase tracking-widest rounded-full mb-4">Enterprise</div>
                <h3 className="text-2xl font-semibold tracking-tight mb-4 text-white">Trellis Terminal</h3>
                
                <div className="text-xs text-emerald-400/70 mb-1 font-mono uppercase tracking-widest">Hardware Deposit</div>
                <div className="text-3xl font-mono tracking-tight mb-4 text-emerald-400">$12,500</div>
                
                <div className="text-xs text-emerald-400/70 mb-1 font-mono uppercase tracking-widest">Annual License</div>
                <div className="text-3xl font-mono tracking-tight mb-4 text-emerald-400">$9,600<span className="text-sm font-mono text-emerald-400/60">/yr</span></div>
                
                <p className="text-sm font-light text-blue-200 leading-relaxed mb-6 mt-6">
                  Air-gapped physical terminal for estate planners and law firms. Fiduciary-grade continuity with zero browser exposure.
                </p>
              </div>
              <div className="mt-4 pt-6 border-t border-emerald-500/20">
                <a href="mailto:trellisvault@gmail.com" className="inline-block w-full text-center text-sm font-bold text-black bg-emerald-400 hover:bg-emerald-300 py-4 rounded-full transition-colors uppercase tracking-widest">Contact Institutional Sales &rarr;</a>
              </div>
            </div>
            <div className="lg:w-2/3 border-t lg:border-t-0 lg:border-l border-emerald-500/20 pt-8 lg:pt-0 lg:pl-12">
              <h4 className="text-lg font-medium text-white mb-6">What's included in the Institutional Tier:</h4>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
                <ul className="space-y-4 text-sm text-blue-200">
                  <li className="flex items-start gap-3"><span className="text-emerald-400 mt-0.5">✓</span> 1x Trellis Hardware Terminal (12.9" touchscreen, biometric TPM, read-only Linux OS)</li>
                  <li className="flex items-start gap-3"><span className="text-emerald-400 mt-0.5">✓</span> 5x NFC Black Cards for multi-sig approvals</li>
                  <li className="flex items-start gap-3"><span className="text-emerald-400 mt-0.5">✓</span> Software license for 1 estate/trust structure</li>
                  <li className="flex items-start gap-3"><span className="text-emerald-400 mt-0.5">✓</span> Quarterly security audits & firmware updates</li>
                </ul>
                <ul className="space-y-4 text-sm text-blue-200">
                  <li className="flex items-start gap-3"><span className="text-emerald-400 mt-0.5">✓</span> Business-hours support (8am–8pm EST)</li>
                  <li className="flex items-start gap-3"><span className="text-emerald-400 mt-0.5">✓</span> Next-business-day device replacement</li>
                  <li className="flex items-start gap-3"><span className="text-emerald-400 mt-0.5">✓</span> API access for practice management integration</li>
                  <li className="flex items-start gap-3"><span className="text-emerald-400 mt-0.5">✓</span> White-glove onboarding and key generation ceremony</li>
                </ul>
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
                <li><Link href="/#features" className="hover:text-emerald-400 transition-colors">Features</Link></li>
                <li><Link href="/#hardware" className="hover:text-emerald-400 transition-colors">Hardware</Link></li>
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
