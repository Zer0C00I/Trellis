import Link from 'next/link'
import { PROGRAM_ID, USDC_MINT } from '@/config/solana'

export default function DocsPage() {
  return (
    <div className="min-h-screen bg-[#020202] text-slate-300 font-sans flex flex-col md:flex-row selection:bg-emerald-500/30">
      <div className="md:hidden flex items-center justify-between p-4 border-b border-white/10 bg-black sticky top-0 z-50">
        <div className="flex items-center gap-3">
          <Link href="/" className="text-slate-400 hover:text-white transition-colors">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="m15 18-6-6 6-6" />
            </svg>
          </Link>
          <Link href="/" className="font-bold text-white tracking-tight flex items-center gap-2">
            <span className="w-3 h-3 bg-emerald-500 rounded-sm"></span>
            Trellis
          </Link>
        </div>
        <span className="text-xs uppercase tracking-widest text-slate-500">Docs</span>
      </div>

      <aside className="w-full md:w-72 md:fixed md:inset-y-0 md:left-0 bg-[#050505] border-r border-white/10 flex flex-col">
        <div className="p-8 hidden md:block border-b border-white/10 sticky top-0 bg-[#050505] z-10 flex-shrink-0">
          <Link
            href="/"
            className="font-bold text-white tracking-tight text-xl flex items-center gap-3 hover:opacity-80 transition-opacity"
          >
            <span className="w-4 h-4 bg-emerald-500 rounded-sm shadow-[0_0_10px_rgba(16,185,129,0.5)]"></span>
            Trellis Docs
          </Link>
        </div>

        <nav className="p-8 space-y-10 flex-grow overflow-y-auto">
          <div>
            <h4 className="text-xs font-bold uppercase tracking-widest text-emerald-500 mb-4">Overview</h4>
            <ul className="space-y-3 text-sm font-medium">
              <li>
                <Link href="#introduction" className="text-slate-400 hover:text-white transition-colors">
                  Introduction
                </Link>
              </li>
              <li>
                <Link href="#status" className="text-slate-400 hover:text-white transition-colors">
                  Status and limits
                </Link>
              </li>
            </ul>
          </div>
          <div>
            <h4 className="text-xs font-bold uppercase tracking-widest text-emerald-500 mb-4">Kiosk</h4>
            <ul className="space-y-3 text-sm font-medium">
              <li>
                <Link href="#kiosk" className="text-slate-400 hover:text-white transition-colors">
                  The Trellis Kiosk
                </Link>
              </li>
            </ul>
          </div>
          <div>
            <h4 className="text-xs font-bold uppercase tracking-widest text-emerald-500 mb-4">Protocol</h4>
            <ul className="space-y-3 text-sm font-medium">
              <li>
                <Link href="#architecture" className="text-slate-400 hover:text-white transition-colors">
                  Vault accounts
                </Link>
              </li>
              <li>
                <Link href="#payees" className="text-slate-400 hover:text-white transition-colors">
                  Payees and the 48h wait
                </Link>
              </li>
              <li>
                <Link href="#guardians" className="text-slate-400 hover:text-white transition-colors">
                  Guardians
                </Link>
              </li>
              <li>
                <Link href="#limits" className="text-slate-400 hover:text-white transition-colors">
                  Spending limits
                </Link>
              </li>
              <li>
                <Link href="#recovery" className="text-slate-400 hover:text-white transition-colors">
                  Key recovery
                </Link>
              </li>
            </ul>
          </div>
        </nav>

        <div className="p-8 border-t border-white/10 bg-[#050505] hidden md:block mt-auto flex-shrink-0">
          <Link
            href="/"
            className="flex items-center justify-center gap-2 w-full py-3 px-4 rounded-xl bg-white/5 hover:bg-white/10 border border-white/10 text-sm font-bold text-white transition-all group"
          >
            Back to Website
          </Link>
        </div>
      </aside>

      <main className="flex-1 md:ml-72 p-6 md:p-16 lg:px-24 lg:py-20 overflow-x-hidden">
        <div className="max-w-4xl mx-auto space-y-24">
          <header id="introduction" className="space-y-6 scroll-mt-32">
            <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-emerald-500/10 border border-emerald-500/20 text-xs font-bold text-emerald-400 uppercase tracking-widest mb-2">
              Prototype · Solana Devnet
            </div>
            <h1 className="text-4xl md:text-6xl font-bold tracking-tight text-white leading-tight">
              Trellis Documentation
            </h1>
            <p className="text-xl md:text-2xl text-slate-400 font-light leading-relaxed max-w-2xl">
              How the kiosk, the family app and the on-chain program fit together, and what each rule actually checks.
            </p>
          </header>

          <hr className="border-white/10" />

          <section id="status" className="space-y-6 scroll-mt-32">
            <h2 className="text-3xl md:text-4xl font-bold tracking-tight text-white">Status and limits</h2>
            <p className="text-slate-300 leading-relaxed font-light">
              Trellis is a hackathon prototype. It runs on Solana Devnet with test USDC only and has not been security
              audited. Do not use it with real funds.
            </p>
            <p className="text-slate-300 leading-relaxed font-light">
              Program ID: <code>{PROGRAM_ID.toBase58()}</code>
            </p>
            <p className="text-slate-300 leading-relaxed font-light">
              Token: Devnet USDC, mint <code>{USDC_MINT.toBase58()}</code>.
            </p>
            <p className="text-slate-300 leading-relaxed font-light">
              Not built yet: the family app (its core library is in progress), notifications, analytics, and any hosted
              service.
            </p>
          </section>

          <hr className="border-white/10" />

          <section id="kiosk" className="space-y-6 scroll-mt-32">
            <h2 className="text-3xl md:text-4xl font-bold tracking-tight text-white">The Trellis Kiosk</h2>
            <p className="text-slate-300 leading-relaxed font-light">
              The kiosk is a single-purpose app for the older person. It is written in Rust with the Slint UI toolkit
              and shows the balance and a short list of approved payees. It cannot open web pages or arbitrary
              addresses.
            </p>
            <p className="text-slate-300 leading-relaxed font-light">
              On first run the device creates its own keypair and stores it locally in a plain key file. There is no
              seed phrase and no wallet import. In this prototype the file is not encrypted by the app and there is no
              secure element or biometric hardware, so physical access to the device matters.
            </p>
            <p className="text-slate-300 leading-relaxed font-light">
              The device image (Alpine Linux with the cage kiosk compositor) is planned. During first-run setup the
              kiosk receives one family member&apos;s address over Bluetooth Low Energy and registers them as payee and
              guardian. This is the only time the kiosk itself adds a person; every later addition is done from the
              family side.
            </p>
            <p className="text-slate-300 leading-relaxed font-light">
              The kiosk needs an internet connection to talk to a Solana RPC node. It is not air-gapped.
            </p>
          </section>

          <hr className="border-white/10" />

          <section id="architecture" className="space-y-6 scroll-mt-32">
            <h2 className="text-3xl md:text-4xl font-bold tracking-tight text-white">Vault accounts</h2>
            <p className="text-slate-300 leading-relaxed font-light">
              All state lives on-chain in the family-vault Anchor program. A vault is a PDA derived from the
              owner&apos;s key and a kind, <code>personal</code> or <code>pool</code>. Each vault has a beneficiary list
              (up to 10 payees) and, once guardians are added, a list of up to 5 trusted approvers.
            </p>
            <p className="text-slate-300 leading-relaxed font-light">
              The kiosk and the family app never talk to each other directly; they only read and write this state.
            </p>
            <div className="bg-black border border-white/10 p-6 rounded-xl font-mono text-sm overflow-x-auto shadow-2xl">
              <pre className="text-slate-300">{`// programs/family-vault/src/state.rs
pub const MAX_BENEFICIARIES: usize = 10;
pub const MAX_APPROVERS: usize = 5;
pub const BENEFICIARY_COOLDOWN_SECS: i64 = 172_800; // 48h
pub const ROTATION_TIMELOCK_SECS: i64 = 259_200;    // 72h

#[account]
pub struct Vault {
    pub owner: Pubkey,
    pub vault_token_account: Pubkey,
    pub daily_limit: u64,      // u64::MAX on a personal vault
    pub spent_today: u64,
    pub last_reset_day: i64,   // unix day, reset lazily on first spend
    pub is_pool: bool,
    pub bump: u8,
    pub seed_owner: Pubkey,    // original owner; fixes the PDA address
}`}</pre>
            </div>
          </section>

          <hr className="border-white/10" />

          <section id="payees" className="space-y-6 scroll-mt-32">
            <h2 className="text-3xl md:text-4xl font-bold tracking-tight text-white">Payees and the 48-hour wait</h2>
            <p className="text-slate-300 leading-relaxed font-light">
              Withdrawals only go to an address on the vault&apos;s beneficiary list. When the owner adds a payee, it
              becomes usable 48 hours later (<code>active_after</code>). Until then a payment to it fails.
            </p>
            <p className="text-slate-300 leading-relaxed font-light">
              To skip the wait, two different trusted approvers must co-sign <code>add_beneficiary_urgent</code> in the
              same transaction. The payee is then usable immediately.
            </p>
          </section>

          <hr className="border-white/10" />

          <section id="guardians" className="space-y-6 scroll-mt-32">
            <h2 className="text-3xl md:text-4xl font-bold tracking-tight text-white">Guardians</h2>
            <p className="text-slate-300 leading-relaxed font-light">
              A vault can have up to 5 trusted approvers (guardians). The owner cannot add themselves. On a personal
              vault, adding a second or later guardian requires an existing guardian to co-sign, so a single compromised
              owner session cannot take over alone.
            </p>
            <p className="text-slate-300 leading-relaxed font-light">
              Guardians approve withdrawals that exceed a pool&apos;s daily limit. The owner never approves their own
              request.
            </p>
          </section>

          <hr className="border-white/10" />

          <section id="limits" className="space-y-6 scroll-mt-32">
            <h2 className="text-3xl md:text-4xl font-bold tracking-tight text-white">Spending limits</h2>
            <p className="text-slate-300 leading-relaxed font-light">
              A personal vault has no spending limit: <code>daily_limit</code> is set to the maximum value and the
              controls are on who can be paid. A family pool has a daily limit, tracked against the on-chain clock by
              day.
            </p>
            <p className="text-slate-300 leading-relaxed font-light">
              A pool withdrawal within the limit goes through. For an amount above it, the owner calls{' '}
              <code>request_withdrawal</code>, which creates a pending request, and one guardian calls{' '}
              <code>approve_withdrawal</code> to release the funds.
            </p>
          </section>

          <hr className="border-white/10" />

          <section id="recovery" className="space-y-6 scroll-mt-32">
            <h2 className="text-3xl md:text-4xl font-bold tracking-tight text-white">Key recovery</h2>
            <p className="text-slate-300 leading-relaxed font-light">
              If the owner&apos;s key is lost or compromised, a trusted approver can call{' '}
              <code>request_owner_rotation</code> with a new owner key. It becomes executable after 72 hours; until then
              the owner or any guardian can cancel it. After the delay anyone can call{' '}
              <code>execute_owner_rotation</code>. A guardian cannot rotate ownership to themselves.
            </p>
          </section>
        </div>
      </main>
    </div>
  )
}
