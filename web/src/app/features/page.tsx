import Image from 'next/image';
import Link from 'next/link';

export default function FeaturesPage() {
  return (
    <div className="min-h-screen bg-black font-sans selection:bg-blue-500/30 selection:text-blue-200">
      
      {/* Global Navigation */}
      <nav className="fixed top-6 left-1/2 -translate-x-1/2 w-[95%] max-w-6xl rounded-full bg-black/40 backdrop-blur-2xl border border-white/10 z-50 px-6 py-4 flex justify-between items-center shadow-[0_0_40px_-10px_rgba(0,0,0,0.5)]">
        <Link href="/" className="flex items-center gap-2">
          <Image src="/logo.png" alt="Trellis" width={240} height={80} className="h-20 w-auto -my-6 invert mix-blend-screen" />
        </Link>
        <div className="hidden md:flex items-center gap-8 text-sm font-medium text-gray-300">
          <Link href="/features" className="text-white transition-colors">Features</Link>
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

      {/* Header */}
      <section className="relative bg-black text-white pt-48 pb-16 px-8 overflow-hidden flex flex-col items-center text-center">
        <div className="absolute top-[-10%] left-1/2 -translate-x-1/2 w-[800px] h-[600px] bg-gradient-to-r from-blue-900/40 via-purple-900/20 to-black blur-[120px] pointer-events-none"></div>
        
        <div className="relative z-10 max-w-5xl mx-auto flex flex-col items-center">
          <h1 className="text-5xl md:text-7xl font-bold tracking-tight text-white mb-6 leading-tight">
            Cryptographically enforced.<br />
            No trusted third parties.
          </h1>
          <p className="text-xl text-slate-400 max-w-2xl mx-auto font-light leading-relaxed mb-12">
            Trellis replaces the entire legacy estate planning stack with immutable Solana smart contracts. Your rules, mathematically guaranteed.
          </p>
        </div>
      </section>

      {/* Core Protocol Features */}
      <section className="bg-white text-black py-32 px-8 relative z-10">
        <div className="max-w-[1200px] mx-auto">
          
          {/* Feature 1: Inheritance */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-16 mb-32 items-center">
            <div>
              <span className="text-xs font-mono font-bold uppercase tracking-widest text-emerald-500 mb-6 block">
                [ BLOCK: 11048A ] INHERITANCE
              </span>
              <h2 className="text-4xl md:text-5xl font-bold tracking-tight mb-6">Time-locked transfers and dead-man's switches.</h2>
              <p className="text-lg text-slate-600 leading-relaxed font-medium mb-6">
                Trellis continuously monitors your on-chain heartbeat. If you stop interacting with your vault for a predefined period, your assets automatically begin a time-locked transition to your designated beneficiaries.
              </p>
              <ul className="space-y-4 text-slate-700 font-medium">
                <li className="flex items-center gap-3"><span className="w-1.5 h-1.5 rounded-full bg-emerald-500"></span> Fully automated, trustless execution</li>
                <li className="flex items-center gap-3"><span className="w-1.5 h-1.5 rounded-full bg-emerald-500"></span> Customizable heartbeat periods (e.g. 1 year)</li>
                <li className="flex items-center gap-3"><span className="w-1.5 h-1.5 rounded-full bg-emerald-500"></span> Zero probate delays, zero legal friction</li>
              </ul>
            </div>
            <div className="bg-slate-50 border border-slate-200 p-8 rounded-3xl shadow-2xl relative overflow-hidden h-[400px] flex items-center justify-center">
               <div className="absolute inset-0 opacity-[0.03] bg-[radial-gradient(circle_at_center,_var(--tw-gradient-stops))] from-black to-transparent bg-[length:20px_20px]" style={{ backgroundImage: 'radial-gradient(circle, #000 1px, transparent 1px)' }}></div>
               <div className="bg-white border border-emerald-500/20 shadow-xl rounded-2xl p-6 w-full max-w-sm relative z-10">
                  <div className="text-xs font-mono text-slate-400 mb-4 uppercase">Heartbeat Status</div>
                  <div className="text-4xl font-mono text-emerald-500 mb-2">ACTIVE</div>
                  <div className="w-full bg-slate-100 h-2 rounded-full mb-4">
                    <div className="bg-emerald-500 h-full rounded-full w-[85%]"></div>
                  </div>
                  <div className="text-xs text-slate-500 font-medium">Time until unlock: 342 Days</div>
               </div>
            </div>
          </div>

          {/* Feature 2: Multi-Sig */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-16 mb-32 items-center">
            <div className="order-2 md:order-1 bg-[#050505] p-8 rounded-3xl shadow-2xl relative overflow-hidden h-[400px] flex items-center justify-center">
               <div className="bg-black border border-blue-500/30 shadow-[0_0_30px_rgba(59,130,246,0.15)] rounded-2xl p-6 w-full max-w-sm relative z-10">
                  <div className="text-xs font-mono text-slate-400 mb-4 uppercase">Governance Quorum</div>
                  <div className="flex justify-between items-end mb-4">
                    <div className="text-4xl font-mono text-blue-400">3<span className="text-xl text-slate-600">/5</span></div>
                    <div className="text-xs text-blue-400 font-bold px-2 py-1 bg-blue-500/20 rounded">THRESHOLD MET</div>
                  </div>
                  <div className="space-y-2">
                    <div className="flex items-center gap-2"><div className="w-2 h-2 rounded-full bg-blue-500"></div><span className="text-xs font-mono text-white">Wife's Ledger</span></div>
                    <div className="flex items-center gap-2"><div className="w-2 h-2 rounded-full bg-blue-500"></div><span className="text-xs font-mono text-white">Law Firm Kiosk</span></div>
                    <div className="flex items-center gap-2"><div className="w-2 h-2 rounded-full bg-blue-500"></div><span className="text-xs font-mono text-white">Trustee Mobile</span></div>
                    <div className="flex items-center gap-2"><div className="w-2 h-2 rounded-full bg-slate-700"></div><span className="text-xs font-mono text-slate-500">Cold Storage Vault (Pending)</span></div>
                  </div>
               </div>
            </div>
            <div className="order-1 md:order-2">
              <span className="text-xs font-mono font-bold uppercase tracking-widest text-blue-500 mb-6 block">
                [ BLOCK: 22849F ] MULTI-SIG
              </span>
              <h2 className="text-4xl md:text-5xl font-bold tracking-tight mb-6">M-of-N Guardians for supreme fault tolerance.</h2>
              <p className="text-lg text-slate-600 leading-relaxed font-medium mb-6">
                Never rely on a single point of failure. Assign recovery shards to family members, fiduciaries, or cold storage devices. If a key is lost, a threshold of Guardians can restore access.
              </p>
              <ul className="space-y-4 text-slate-700 font-medium">
                <li className="flex items-center gap-3"><span className="w-1.5 h-1.5 rounded-full bg-blue-500"></span> Fully customizable M-of-N rules (e.g. 3 of 5)</li>
                <li className="flex items-center gap-3"><span className="w-1.5 h-1.5 rounded-full bg-blue-500"></span> Geographically distribute your trust</li>
                <li className="flex items-center gap-3"><span className="w-1.5 h-1.5 rounded-full bg-blue-500"></span> Revoke or rotate Guardians at any time</li>
              </ul>
            </div>
          </div>

          {/* Feature 3: Mathematical Limits */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-16 mb-32 items-center">
            <div>
              <span className="text-xs font-mono font-bold uppercase tracking-widest text-purple-500 mb-6 block">
                [ BLOCK: 39482C ] SPENDING LOGIC
              </span>
              <h2 className="text-4xl md:text-5xl font-bold tracking-tight mb-6">Algorithmic allowances and daily limits.</h2>
              <p className="text-lg text-slate-600 leading-relaxed font-medium mb-6">
                Create trust structures that automatically dispense funds or strictly enforce daily withdrawal caps. Any transaction attempting to exceed the limit is programmatically blocked at the RPC level.
              </p>
              <ul className="space-y-4 text-slate-700 font-medium">
                <li className="flex items-center gap-3"><span className="w-1.5 h-1.5 rounded-full bg-purple-500"></span> Stop rapid asset draining instantly</li>
                <li className="flex items-center gap-3"><span className="w-1.5 h-1.5 rounded-full bg-purple-500"></span> Configure monthly allowances for beneficiaries</li>
                <li className="flex items-center gap-3"><span className="w-1.5 h-1.5 rounded-full bg-purple-500"></span> Require multi-sig overrides for large transfers</li>
              </ul>
            </div>
            <div className="bg-slate-50 border border-slate-200 p-8 rounded-3xl shadow-2xl relative overflow-hidden h-[400px] flex items-center justify-center">
               <div className="bg-white border border-purple-500/20 shadow-xl rounded-2xl p-6 w-full max-w-sm relative z-10">
                  <div className="text-xs font-mono text-slate-400 mb-4 uppercase">Daily Limit Utilization</div>
                  <div className="text-4xl font-mono text-purple-500 mb-2">$8,500 <span className="text-xl text-slate-400">/ $10k</span></div>
                  <div className="w-full bg-slate-100 h-2 rounded-full mb-4 flex">
                    <div className="bg-purple-500 h-full rounded-l-full w-[85%]"></div>
                    <div className="bg-red-500 h-full rounded-r-full w-[15%]"></div>
                  </div>
                  <div className="text-xs text-red-500 font-bold uppercase tracking-widest flex items-center gap-2">
                    <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" /></svg>
                    TX Blocked: Limit Exceeded
                  </div>
               </div>
            </div>
          </div>

          {/* Feature 4: DeFi Composability */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-16 mb-32 items-center">
            <div className="order-2 md:order-1 bg-[#050505] p-8 rounded-3xl shadow-2xl relative overflow-hidden h-[400px] flex items-center justify-center">
               <div className="bg-black border border-amber-500/30 shadow-[0_0_30px_rgba(245,158,11,0.15)] rounded-2xl p-6 w-full max-w-sm relative z-10">
                  <div className="text-xs font-mono text-slate-400 mb-4 uppercase">Estate Yield Accrual</div>
                  <div className="flex justify-between items-end mb-4">
                    <div className="text-4xl font-mono text-amber-400">4.82%<span className="text-xl text-slate-600"> APY</span></div>
                  </div>
                  <div className="space-y-4">
                    <div className="flex items-center justify-between border-b border-white/5 pb-2">
                      <div className="flex items-center gap-2"><div className="w-6 h-6 rounded-full bg-blue-500/20 flex items-center justify-center text-[10px] text-blue-400 font-bold">SOL</div><span className="text-sm font-mono text-white">Staked SOL</span></div>
                      <div className="text-sm font-mono text-emerald-400">+ 7.2% APY</div>
                    </div>
                    <div className="flex items-center justify-between border-b border-white/5 pb-2">
                      <div className="flex items-center gap-2"><div className="w-6 h-6 rounded-full bg-emerald-500/20 flex items-center justify-center text-[10px] text-emerald-400 font-bold">USDC</div><span className="text-sm font-mono text-white">Lending Pool</span></div>
                      <div className="text-sm font-mono text-emerald-400">+ 3.5% APY</div>
                    </div>
                  </div>
               </div>
            </div>
            <div className="order-1 md:order-2">
              <span className="text-xs font-mono font-bold uppercase tracking-widest text-amber-500 mb-6 block">
                [ BLOCK: 83991C ] COMPOSABILITY
              </span>
              <h2 className="text-4xl md:text-5xl font-bold tracking-tight mb-6">Yield-bearing estates plugged into DeFi.</h2>
              <p className="text-lg text-slate-600 leading-relaxed font-medium mb-6">
                Your vault isn't just cold storage. Trellis natively plugs into the Solana DeFi ecosystem. Let your generational wealth safely earn yield through staking and over-collateralized lending while it sits securely locked in your trust.
              </p>
              <ul className="space-y-4 text-slate-700 font-medium">
                <li className="flex items-center gap-3"><span className="w-1.5 h-1.5 rounded-full bg-amber-500"></span> Native liquid staking integration</li>
                <li className="flex items-center gap-3"><span className="w-1.5 h-1.5 rounded-full bg-amber-500"></span> Auto-compound wealth for beneficiaries</li>
                <li className="flex items-center gap-3"><span className="w-1.5 h-1.5 rounded-full bg-amber-500"></span> Strict whitelists block interaction with risky protocols</li>
              </ul>
            </div>
          </div>

          {/* Feature 5: Speed & Exemption */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-16 items-center">
            <div>
              <span className="text-xs font-mono font-bold uppercase tracking-widest text-blue-600 mb-6 block">
                [ BLOCK: 99100Z ] INFRASTRUCTURE
              </span>
              <h2 className="text-4xl md:text-5xl font-bold tracking-tight mb-6">Zero delays. Permanent existence.</h2>
              <p className="text-lg text-slate-600 leading-relaxed font-medium mb-6">
                Built natively on Solana, Trellis settles multi-million dollar estate transfers globally in 400 milliseconds. More importantly, your vault is funded with permanent rent exemption, ensuring it lives on the blockchain forever without maintenance fees.
              </p>
              <ul className="space-y-4 text-slate-700 font-medium">
                <li className="flex items-center gap-3"><span className="w-1.5 h-1.5 rounded-full bg-blue-600"></span> Sub-second global settlement</li>
                <li className="flex items-center gap-3"><span className="w-1.5 h-1.5 rounded-full bg-blue-600"></span> Fractions of a penny in transaction fees</li>
                <li className="flex items-center gap-3"><span className="w-1.5 h-1.5 rounded-full bg-blue-600"></span> Permanent rent-exemption guarantees</li>
              </ul>
            </div>
            <div className="bg-slate-50 border border-slate-200 p-8 rounded-3xl shadow-2xl relative overflow-hidden h-[400px] flex items-center justify-center">
               <div className="absolute inset-0 bg-gradient-to-br from-blue-500/5 to-transparent pointer-events-none"></div>
               <div className="bg-white border border-blue-500/20 shadow-xl rounded-2xl p-6 w-full max-w-sm relative z-10 text-center">
                  <svg className="w-16 h-16 text-blue-500 mx-auto mb-6" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1} d="M13 10V3L4 14h7v7l9-11h-7z" /></svg>
                  <div className="text-xs font-mono text-slate-400 mb-2 uppercase">Settlement Speed</div>
                  <div className="text-5xl font-mono text-black font-bold mb-6">400<span className="text-2xl text-slate-400">ms</span></div>
                  <div className="text-sm text-slate-500 font-medium bg-slate-100 py-2 rounded-lg">Status: Finalized</div>
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
