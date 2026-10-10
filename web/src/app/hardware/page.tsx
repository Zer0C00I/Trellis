import Image from 'next/image';
import Link from 'next/link';

export default function HardwarePage() {
  return (
    <div className="min-h-screen bg-black font-sans selection:bg-emerald-500/30 selection:text-emerald-200">
      
      {/* Global Navigation */}
      <nav className="fixed top-6 left-1/2 -translate-x-1/2 w-[95%] max-w-6xl rounded-full bg-black/40 backdrop-blur-2xl border border-white/10 z-50 px-6 py-4 flex justify-between items-center shadow-[0_0_40px_-10px_rgba(0,0,0,0.5)]">
        <Link href="/" className="flex items-center gap-2">
          <Image src="/logo.png" alt="Trellis" width={240} height={80} className="h-20 w-auto -my-6 invert mix-blend-screen" />
        </Link>
        <div className="hidden md:flex items-center gap-8 text-sm font-medium text-gray-300">
          <Link href="/features" className="hover:text-white transition-colors">Features</Link>
          <Link href="/hardware" className="text-white transition-colors">Hardware</Link>
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
        <div className="absolute top-[-10%] left-1/2 -translate-x-1/2 w-[800px] h-[600px] bg-gradient-to-r from-emerald-900/40 via-green-900/20 to-black blur-[120px] pointer-events-none"></div>
        
        <div className="relative z-10 max-w-5xl mx-auto flex flex-col items-center">
          <h1 className="text-5xl md:text-7xl font-bold tracking-tight text-white mb-6 leading-tight">
            Zero browser exposure.<br />
            Absolute on-chain rules.
          </h1>
          <p className="text-xl text-slate-400 max-w-2xl mx-auto font-light leading-relaxed mb-12">
            The Trellis Terminal is a dedicated touchscreen kiosk for the vault holder. It shows the balance and sends USDC to approved family. Nothing else.
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
                  <div>TRELLIS OS v2.1.0</div>
                  <div className="flex gap-4">
                    <span className="text-emerald-500">SOLANA DEVNET</span>
                    <span>PAIRED</span>
                    <span>14:32</span>
                  </div>
                </div>
                {/* Main UI */}
                <div className="flex-grow flex items-center justify-center relative p-8">
                  {/* Pairing graphic */}
                  <div className="w-32 h-32 border border-emerald-500/30 rounded-full flex items-center justify-center animate-[pulse_4s_ease-in-out_infinite]">
                    <div className="w-24 h-24 border border-emerald-500/50 rounded-full flex items-center justify-center">
                      <div className="w-16 h-16 bg-emerald-500/20 rounded-full blur-md"></div>
                    </div>
                  </div>
                  <div className="absolute bottom-16 text-center">
                    <h3 className="text-white font-medium text-lg tracking-wide mb-2">Awaiting Guardian Approval</h3>
                    <p className="text-slate-500 font-mono text-xs uppercase tracking-widest">Over daily limit · 250 USDC to Daughter</p>
                  </div>
                </div>
              </div>
            </div>
            {/* Status light */}
            <div className="absolute -bottom-4 right-12 w-32 h-8 bg-black border border-white/10 rounded-t-xl flex justify-center pt-2">
              <div className="w-16 h-1 bg-emerald-500/50 rounded-full animate-pulse"></div>
            </div>
          </div>
        </div>
      </section>

      {/* Hardware Specs */}
      <section className="bg-white text-black py-32 px-8">
        <div className="max-w-[1200px] mx-auto">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-16 mb-24">
            <div>
              <span className="text-xs font-mono font-bold uppercase tracking-widest text-emerald-500 mb-6 block">
                [ BLOCK: 910283B4C ] KIOSK
              </span>
              <h2 className="text-4xl md:text-5xl font-bold tracking-tight mb-6">Why a kiosk?</h2>
              <p className="text-lg text-slate-600 leading-relaxed font-medium mb-6">
                Browser wallets hand the most vulnerable user the most complex tool: extensions, seed phrases, and phishing links one tap away. 
                <br /><br />
                The Trellis Terminal strips that away. Two screens, large type, one action: send to family. The Solana program enforces every rule underneath it.
              </p>
            </div>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
              <div className="bg-slate-50 border border-slate-200 p-8 rounded-3xl">
                <div className="w-10 h-10 bg-emerald-100 text-emerald-500 rounded-xl flex items-center justify-center mb-6">
                  <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" /></svg>
                </div>
                <h4 className="font-bold text-lg mb-2">Programmable Inheritance</h4>
                <p className="text-sm text-slate-600">The kiosk can only pay registered beneficiaries. New names wait 48 hours before they can receive funds.</p>
              </div>
              <div className="bg-slate-50 border border-slate-200 p-8 rounded-3xl">
                <div className="w-10 h-10 bg-emerald-100 text-emerald-500 rounded-xl flex items-center justify-center mb-6">
                  <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5.121 17.804A13.937 13.937 0 0112 16c2.5 0 4.847.655 6.879 1.804M15 10a3 3 0 11-6 0 3 3 0 016 0zm6 2a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
                </div>
                <h4 className="font-bold text-lg mb-2">Daily USDC Spending Limits</h4>
                <p className="text-sm text-slate-600">Sends stay under a daily USDC ceiling that the program enforces and resets every UTC day.</p>
              </div>
              <div className="bg-slate-50 border border-slate-200 p-8 rounded-3xl">
                <div className="w-10 h-10 bg-emerald-100 text-emerald-500 rounded-xl flex items-center justify-center mb-6">
                  <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18.364 5.636l-3.536 3.536m0 5.656l3.536 3.536M9.172 9.172L5.636 5.636m3.536 9.192l-3.536 3.536M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-5 0a4 4 0 11-8 0 4 4 0 018 0z" /></svg>
                </div>
                <h4 className="font-bold text-lg mb-2">Multi-Sig Guardian Approvals</h4>
                <p className="text-sm text-slate-600">Anything over the limit waits on-chain until a guardian signs from their own device.</p>
              </div>
              <div className="bg-slate-50 border border-slate-200 p-8 rounded-3xl">
                <div className="w-10 h-10 bg-emerald-100 text-emerald-500 rounded-xl flex items-center justify-center mb-6">
                  <svg className="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" /></svg>
                </div>
                <h4 className="font-bold text-lg mb-2">Tamper Proof</h4>
                <p className="text-sm text-slate-600">Physical intrusion detection instantly zeroes out the secure enclave if the casing is breached.</p>
              </div>
            </div>
          </div>
          
          <div className="mt-16 pt-16 border-t border-slate-200 text-center">
             <h3 className="text-2xl font-bold mb-6">Ready to deploy institutional infrastructure?</h3>
             <Link href="/pricing" className="inline-block bg-emerald-500 hover:bg-emerald-600 text-white font-bold py-4 px-8 rounded-full transition-colors uppercase tracking-widest text-sm">
                View Enterprise Pricing
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
