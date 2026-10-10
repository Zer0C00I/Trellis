"use client";

import Link from 'next/link';
import { motion } from 'framer-motion';
import { useState } from 'react';

function CodeSnippet() {
  const [copied, setCopied] = useState(false);

  const code = `// programs/family-vault/src/state/vault.rs
#[account]
pub struct Vault {
    pub owner: Pubkey,
    pub vault_token_account: Pubkey,
    pub daily_limit: u64,
    pub spent_today: u64,
    pub last_spend_reset: i64,
    pub guardian_threshold: u8,
}`;

  const handleCopy = () => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="relative bg-black border border-white/10 p-6 rounded-xl font-mono text-sm shadow-2xl group">
      <button 
        onClick={handleCopy}
        className="absolute top-4 right-4 p-2 rounded-lg bg-white/5 hover:bg-white/10 border border-white/10 text-slate-400 hover:text-white transition-all opacity-0 group-hover:opacity-100"
      >
        {copied ? (
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="text-emerald-500"><polyline points="20 6 9 17 4 12"></polyline></svg>
        ) : (
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
        )}
      </button>
      <div className="overflow-x-auto">
        <pre className="text-slate-300">
          <span className="text-slate-500">{'// programs/family-vault/src/state/vault.rs'}</span>{"\n"}
          <span className="text-purple-400">#[account]</span>{"\n"}
          <span className="text-emerald-400">pub struct</span> <span className="text-amber-300">Vault</span> {"{"}{"\n"}
          {"    "}<span className="text-emerald-400">pub</span> owner: <span className="text-amber-300">Pubkey</span>,{"\n"}
          {"    "}<span className="text-emerald-400">pub</span> vault_token_account: <span className="text-amber-300">Pubkey</span>,{"\n"}
          {"    "}<span className="text-emerald-400">pub</span> daily_limit: <span className="text-amber-300">u64</span>,{"\n"}
          {"    "}<span className="text-emerald-400">pub</span> spent_today: <span className="text-amber-300">u64</span>,{"\n"}
          {"    "}<span className="text-emerald-400">pub</span> last_spend_reset: <span className="text-amber-300">i64</span>,{"\n"}
          {"    "}<span className="text-emerald-400">pub</span> guardian_threshold: <span className="text-amber-300">u8</span>,{"\n"}
          {"}"}
        </pre>
      </div>
    </div>
  );
}

export default function DocsPage() {
  return (
    <div className="min-h-screen bg-[#020202] text-slate-300 font-sans flex flex-col md:flex-row selection:bg-emerald-500/30">
      
      {/* Mobile Header (Hidden on md+) */}
      <div className="md:hidden flex items-center justify-between p-4 border-b border-white/10 bg-black sticky top-0 z-50">
        <div className="flex items-center gap-3">
          <Link href="/" className="text-slate-400 hover:text-white transition-colors">
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="m15 18-6-6 6-6"/></svg>
          </Link>
          <Link href="/" className="font-bold text-white tracking-tight flex items-center gap-2">
            <span className="w-3 h-3 bg-emerald-500 rounded-sm"></span>
            Trellis
          </Link>
        </div>
        <span className="text-xs uppercase tracking-widest text-slate-500">Docs</span>
      </div>

      {/* Left Sidebar (Fixed) */}
      <aside className="w-full md:w-72 md:fixed md:inset-y-0 md:left-0 bg-[#050505] border-r border-white/10 flex flex-col">
        <div className="p-8 hidden md:block border-b border-white/10 sticky top-0 bg-[#050505] z-10 flex-shrink-0">
          <Link href="/" className="font-bold text-white tracking-tight text-xl flex items-center gap-3 hover:opacity-80 transition-opacity">
            <span className="w-4 h-4 bg-emerald-500 rounded-sm shadow-[0_0_10px_rgba(16,185,129,0.5)]"></span>
            Trellis Docs
          </Link>
        </div>
        
        <nav className="p-8 space-y-10 flex-grow overflow-y-auto">
          <div>
            <h4 className="text-xs font-bold uppercase tracking-widest text-emerald-500 mb-4">Hardware</h4>
            <ul className="space-y-3 text-sm font-medium">
              <li><Link href="#kiosk" className="text-slate-400 hover:text-white transition-colors">Trellis Kiosk Terminal</Link></li>
              <li><Link href="#biometrics" className="text-slate-400 hover:text-white transition-colors">Biometric Authentication</Link></li>
              <li><Link href="#nfc" className="text-slate-400 hover:text-white transition-colors">NFC Black Card Sync</Link></li>
            </ul>
          </div>

          <div>
            <h4 className="text-xs font-bold uppercase tracking-widest text-emerald-500 mb-4">Software & Protocol</h4>
            <ul className="space-y-3 text-sm font-medium">
              <li><Link href="#architecture" className="text-slate-400 hover:text-white transition-colors">Vault Architecture (Solana)</Link></li>
              <li><Link href="#multisig" className="text-slate-400 hover:text-white transition-colors">Guardian Multi-Sig</Link></li>
              <li><Link href="#limits" className="text-slate-400 hover:text-white transition-colors">Daily Spending Limits</Link></li>
            </ul>
          </div>
          
          <div>
            <h4 className="text-xs font-bold uppercase tracking-widest text-emerald-500 mb-4">Enterprise</h4>
            <ul className="space-y-3 text-sm font-medium">
              <li><Link href="#pricing" className="text-slate-400 hover:text-white transition-colors">Pricing & Plans</Link></li>
              <li><Link href="#support" className="text-slate-400 hover:text-white transition-colors">Dedicated Support</Link></li>
            </ul>
          </div>
        </nav>

        <div className="p-8 border-t border-white/10 bg-[#050505] hidden md:block mt-auto flex-shrink-0">
          <Link href="/" className="flex items-center justify-center gap-2 w-full py-3 px-4 rounded-xl bg-white/5 hover:bg-white/10 border border-white/10 text-sm font-bold text-white transition-all group">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="group-hover:-translate-x-1 transition-transform"><path d="m15 18-6-6 6-6"/></svg>
            Back to Website
          </Link>
        </div>
      </aside>

      {/* Main Content Area */}
      <main className="flex-1 md:ml-72 p-6 md:p-16 lg:px-24 lg:py-20 overflow-x-hidden">
        <div className="max-w-4xl mx-auto space-y-24">
          
          {/* Header */}
          <motion.header 
            id="introduction" 
            className="space-y-6 scroll-mt-32"
            initial={{ opacity: 0, y: 30 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, ease: "easeOut" }}
          >
            <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-emerald-500/10 border border-emerald-500/20 text-xs font-bold text-emerald-400 uppercase tracking-widest mb-2">
              v2.0 Protocol Reference
            </div>
            <h1 className="text-4xl md:text-6xl font-bold tracking-tight text-white leading-tight">
              Trellis Infrastructure
            </h1>
            <p className="text-xl md:text-2xl text-slate-400 font-light leading-relaxed max-w-2xl">
              The official documentation for the Trellis Kiosk ecosystem, Solana smart contract protocol, and enterprise deployment guidelines.
            </p>
          </motion.header>

          <hr className="border-white/10" />

          {/* Section: Hardware Kiosk */}
          <motion.section 
            id="kiosk" 
            className="space-y-8 scroll-mt-32"
            initial={{ opacity: 0, y: 50 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-100px" }}
            transition={{ duration: 0.8 }}
          >
            <h2 className="text-3xl md:text-4xl font-bold tracking-tight text-white">
              The Trellis Kiosk Terminal
            </h2>
            <p className="text-lg text-slate-300 leading-loose font-light">
              Trellis was engineered because browser-based crypto wallets are inherently vulnerable to malware, session hijacking, and phishing. Our flagship product is the <strong>Trellis Kiosk Terminal</strong>, a dedicated, air-gapped hardware device built for law firms, family offices, and wealth managers.
            </p>
            
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mt-8">
              <div className="bg-[#080808] border border-white/10 p-8 rounded-2xl">
                <h3 className="text-white font-bold text-xl mb-3">T-8492 Form Factor</h3>
                <p className="text-slate-400 text-sm leading-relaxed">
                  A sleek, anodized aluminum chassis housing a 12.9&quot; high-resolution touchscreen. The device runs TrellisOS—a highly stripped-down, read-only Linux kernel designed strictly to communicate with the Solana RPC layer.
                </p>
              </div>
              <div className="bg-[#080808] border border-white/10 p-8 rounded-2xl" id="biometrics">
                <h3 className="text-white font-bold text-xl mb-3">Biometric TPM Enclave</h3>
                <p className="text-slate-400 text-sm leading-relaxed">
                  The terminal utilizes a secure hardware enclave to process fingerprint scans. Biometric data never leaves the device; it is used locally to decrypt the master signing key required to approve multi-sig payload transactions on the blockchain.
                </p>
              </div>
            </div>

            <div className="bg-blue-900/10 border border-blue-500/20 p-8 rounded-2xl mt-8" id="nfc">
              <h3 className="text-blue-400 font-bold text-lg mb-3 flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-blue-500 animate-pulse"></span>
                NFC Black Card Sync
              </h3>
              <p className="text-slate-300 leading-relaxed font-light">
                Every Trellis Kiosk comes paired with three Trellis Black Cards. These NFC-enabled physical cards act as physical 2FA markers. To authorize a withdrawal exceeding the daily limit, the primary executor must tap the Black Card on the terminal reader while providing biometric authorization.
              </p>
            </div>
          </motion.section>

          <hr className="border-white/10" />

          {/* Section: Protocol Architecture */}
          <motion.section 
            id="architecture" 
            className="space-y-8 scroll-mt-32"
            initial={{ opacity: 0, y: 50 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-100px" }}
            transition={{ duration: 0.8 }}
          >
            <h2 className="text-3xl md:text-4xl font-bold tracking-tight text-white">
              Smart Contract Protocol
            </h2>
            <p className="text-lg text-slate-300 leading-loose font-light">
              While the hardware ensures physical security, the logic is enforced entirely by our audited Anchor/Rust smart contracts deployed on Solana. The Kiosk merely acts as the interface. The Trellis protocol guarantees that trust rules are executed purely via mathematical consensus.
            </p>
            
            <h3 className="text-xl font-bold text-white mt-12 mb-4">Vault PDA State</h3>
            <CodeSnippet />

            <div className="space-y-12 mt-12">
              <div id="multisig">
                <h4 className="text-lg font-bold text-white mb-2">Guardian Multi-Sig Architecture</h4>
                <p className="text-slate-400 leading-relaxed font-light">
                  When a transaction exceeds the predefined limit (or during a beneficiary update), the transaction enters a pending state on-chain. It requires <code>m-of-n</code> cryptographic approvals from whitelisted Guardian pubkeys. Guardians can approve remotely via their own wallets, or directly through an authorized Trellis Kiosk.
                </p>
              </div>

              <div id="limits">
                <h4 className="text-lg font-bold text-white mb-2">Mathematical Daily Limits</h4>
                <p className="text-slate-400 leading-relaxed font-light">
                  Spending limits are not enforced by an API—they are verified against the Solana <code>Clock::get()?.unix_timestamp</code> in the smart contract. The contract checks if the requested withdrawal amount plus the <code>spent_today</code> exceeds the <code>daily_limit</code>. If 24 hours have passed since the <code>last_spend_reset</code>, the limit automatically resets.
                </p>
              </div>
            </div>
          </motion.section>

          <hr className="border-white/10" />

          {/* Section: Pricing & Enterprise */}
          <motion.section 
            id="pricing" 
            className="space-y-8 scroll-mt-32 pb-24"
            initial={{ opacity: 0, y: 50 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-100px" }}
            transition={{ duration: 0.8 }}
          >
            <h2 className="text-3xl md:text-4xl font-bold tracking-tight text-white">
              Pricing & Enterprise Deployment
            </h2>
            <p className="text-lg text-slate-300 leading-loose font-light">
              Trellis offers flexible deployment models. Whether you are an individual wanting a digital-only vault, or a major family office deploying dedicated hardware across multiple global estates, our pricing is transparent.
            </p>

            <div className="mt-12 space-y-12">
              {/* For Individuals Section */}
              <div>
                <h4 className="text-xl font-bold text-white mb-6 border-b border-white/10 pb-4">For Individuals</h4>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                  {/* Digital-Only Free */}
                  <div className="bg-[#050505] border border-white/10 rounded-3xl p-6 flex flex-col">
                    <h3 className="text-lg font-bold text-white mb-2">Trellis Personal</h3>
                    <div className="text-2xl font-mono text-emerald-400 mb-6">Free</div>
                    <p className="text-slate-400 font-light text-sm mb-6 flex-grow">
                      Essential self-custody. Not for institutional or high-value estates.
                    </p>
                    <ul className="space-y-3 text-xs text-slate-300">
                      <li className="flex items-start gap-3">
                        <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 mt-1"></span>
                        Standard Guardian logic (Max 2)
                      </li>
                      <li className="flex items-start gap-3">
                        <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 mt-1"></span>
                        Web App & Phantom Support
                      </li>
                      <li className="flex items-start gap-3">
                        <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 mt-1"></span>
                        No monthly SaaS fees
                      </li>
                    </ul>
                  </div>

                  {/* Digital-Only Pro */}
                  <div className="bg-blue-900/10 border border-blue-500/30 shadow-[0_0_20px_rgba(59,130,246,0.05)] rounded-3xl p-6 flex flex-col relative overflow-hidden">
                    <h3 className="text-lg font-bold text-white mb-2">Trellis Pro</h3>
                    <div className="text-2xl font-mono text-blue-400 mb-1">$25<span className="text-xs text-blue-400/60">/mo</span></div>
                    <div className="text-[10px] text-blue-400/60 mb-5 font-mono uppercase tracking-widest">Billed Annually</div>
                    <p className="text-slate-400 font-light text-sm mb-6 flex-grow">
                      Advanced governance for active digital estates.
                    </p>
                    <ul className="space-y-3 text-xs text-slate-300">
                      <li className="flex items-start gap-3">
                        <span className="w-1.5 h-1.5 rounded-full bg-blue-500 mt-1"></span>
                        Up to 10 Guardians (M-of-N)
                      </li>
                      <li className="flex items-start gap-3">
                        <span className="w-1.5 h-1.5 rounded-full bg-blue-500 mt-1"></span>
                        SMS & Push Notification Alerts
                      </li>
                      <li className="flex items-start gap-3">
                        <span className="w-1.5 h-1.5 rounded-full bg-blue-500 mt-1"></span>
                        Downloadable Audit Logs
                      </li>
                    </ul>
                  </div>
                  
                  {/* Digital-Only Advisor */}
                  <div className="bg-[#050505] border border-emerald-500/30 rounded-3xl p-6 flex flex-col relative overflow-hidden col-span-1 md:col-span-2">
                    <h3 className="text-lg font-bold text-white mb-2">Trellis Advisor</h3>
                    <div className="text-2xl font-mono text-emerald-400 mb-1">$499<span className="text-xs text-emerald-400/60">/yr</span></div>
                    <div className="text-[10px] text-emerald-400/60 mb-5 font-mono uppercase tracking-widest">Flat Annual Rate</div>
                    <p className="text-slate-400 font-light text-sm mb-6 flex-grow">
                      For estate planners and small law firms managing multiple digital estates.
                    </p>
                    <ul className="space-y-3 text-xs text-slate-300">
                      <li className="flex items-start gap-3">
                        <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 mt-1"></span>
                        Manage up to 50 client estates
                      </li>
                      <li className="flex items-start gap-3">
                        <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 mt-1"></span>
                        Co-branded Vault Interface
                      </li>
                      <li className="flex items-start gap-3">
                        <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 mt-1"></span>
                        Priority API Access
                      </li>
                    </ul>
                  </div>
                </div>
              </div>

              {/* For Institutions Section */}
              <div>
                <h4 className="text-xl font-bold text-emerald-400 mb-6 border-b border-emerald-500/20 pb-4">For Institutions & Family Offices</h4>
                <div className="bg-[#050505] border-2 border-emerald-500/50 shadow-[0_0_30px_rgba(16,185,129,0.1)] rounded-3xl p-8 flex flex-col md:flex-row gap-8 overflow-hidden">
                  <div className="md:w-1/3 flex flex-col">
                    <div className="inline-block self-start px-3 py-1 bg-emerald-500/20 text-emerald-400 text-[10px] font-bold uppercase tracking-widest rounded-full mb-4">
                      Institutional Standard
                    </div>
                    <h3 className="text-lg font-bold text-white mb-4">Trellis Terminal</h3>
                    <div className="text-[10px] font-mono text-emerald-400/60 mb-1 uppercase tracking-widest">Hardware Deposit</div>
                    <div className="text-3xl font-mono text-emerald-400 mb-2">$12,500</div>
                    <div className="text-[10px] font-mono text-emerald-400/60 mb-1 mt-2 uppercase tracking-widest">Annual License</div>
                    <div className="text-3xl font-mono text-emerald-400 mb-2">$9,600<span className="text-lg text-emerald-400/60">/yr</span></div>
                    <p className="text-slate-400 font-light text-sm mb-8 mt-4 flex-grow">
                      Annual platform, support, and replacement license.
                    </p>
                    <a href="mailto:trellisvault@gmail.com" className="inline-block text-center bg-emerald-400 text-black font-bold px-6 py-3 rounded-full text-sm hover:bg-emerald-300 transition-colors">
                      Contact Institutional Sales
                    </a>
                  </div>
                  <div className="md:w-2/3 border-t md:border-t-0 md:border-l border-emerald-500/20 pt-6 md:pt-0 md:pl-8">
                    <p className="text-slate-300 font-light text-sm mb-6">
                      Air-gapped physical terminal for estate planners and law firms. Fiduciary-grade continuity with zero browser exposure.
                    </p>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                      <ul className="space-y-3 text-xs text-slate-300">
                        <li className="flex items-start gap-3">
                          <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 mt-1"></span>
                          2x Trellis Hardware Terminals
                        </li>
                        <li className="flex items-start gap-3">
                          <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 mt-1"></span>
                          20x NFC Trellis Black Cards
                        </li>
                        <li className="flex items-start gap-3">
                          <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 mt-1"></span>
                          License for 5 estate structures
                        </li>
                        <li className="flex items-start gap-3">
                          <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 mt-1"></span>
                          Quarterly security audits
                        </li>
                      </ul>
                      <ul className="space-y-3 text-xs text-slate-300">
                        <li className="flex items-start gap-3">
                          <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 mt-1"></span>
                          24/7 dedicated support
                        </li>
                        <li className="flex items-start gap-3">
                          <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 mt-1"></span>
                          Same-Day replacement SLA
                        </li>
                        <li className="flex items-start gap-3">
                          <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 mt-1"></span>
                          API access for integrations
                        </li>
                      </ul>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </motion.section>

        </div>
      </main>

    </div>
  );
}
