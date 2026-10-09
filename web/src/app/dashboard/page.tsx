'use client';
import { useState, useEffect, useCallback } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { LAMPORTS_PER_SOL, PublicKey, SystemProgram } from '@solana/web3.js';
import { BN, ProgramAccount } from '@coral-xyz/anchor';
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { BeneficiaryTable } from '../../components/dashboard/BeneficiaryTable';
import { AddBeneficiaryModal } from '../../components/dashboard/AddBeneficiaryModal';
import { AddApproverModal } from '../../components/dashboard/AddApproverModal';
import { DepositUsdcModal } from '../../components/dashboard/DepositUsdcModal';
import { Button } from '../../components/ui/button';
import { Toaster, toast } from 'sonner';
import { useAnchorProvider } from '../../config/useAnchorProvider';
import { USDC_MINT, USDC_DECIMALS, explorerTxUrl, explorerAddressUrl } from '../../config/solana';
import { Loader2 } from 'lucide-react';

// Rent for vault + beneficiary list + vault token account is ~0.011 SOL; keep headroom for fees.
const MIN_SOL_FOR_INIT = 0.02;
const U64_MAX = new BN('18446744073709551615');

function formatUsdc(units: BN): string {
  const s = units.toString().padStart(USDC_DECIMALS + 1, '0');
  const whole = s.slice(0, -USDC_DECIMALS);
  const frac = s.slice(-USDC_DECIMALS).slice(0, 2);
  return `$${Number(whole).toLocaleString()}.${frac}`;
}

interface VaultSummary {
  dailyLimit: BN;
  spentToday: BN;
  isPool: boolean;
  vaultTokenAccount: PublicKey;
  balance: BN;
}

export default function Dashboard() {
  const { connected } = useWallet();
  const { program, provider } = useAnchorProvider();

  const [isApproverModalOpen, setIsApproverModalOpen] = useState(false);
  const [isDepositOpen, setIsDepositOpen] = useState(false);
  const [refreshCounter, setRefreshCounter] = useState(0);
  const refresh = useCallback(() => setRefreshCounter((c) => c + 1), []);

  const [vaultExists, setVaultExists] = useState<boolean | null>(null);
  const [vaultPda, setVaultPda] = useState<PublicKey | null>(null);
  const [summary, setSummary] = useState<VaultSummary | null>(null);
  const [initializing, setInitializing] = useState(false);
  const [airdropping, setAirdropping] = useState(false);
  const [solBalance, setSolBalance] = useState<number | null>(null);
  const [programDeployed, setProgramDeployed] = useState<boolean | null>(null);

  const [pendingRequests, setPendingRequests] = useState<ProgramAccount[]>([]);

  useEffect(() => {
    if (!program || !provider) return;
    let cancelled = false;

    const load = async () => {
      try {
        const owner = provider.wallet.publicKey;
        const [pda] = PublicKey.findProgramAddressSync(
          [Buffer.from('vault'), owner.toBuffer(), Buffer.from('personal')],
          program.programId
        );

        const [programInfo, lamports, vault] = await Promise.all([
          provider.connection.getAccountInfo(program.programId),
          provider.connection.getBalance(owner),
          program.account.vault.fetchNullable(pda),
        ]);
        if (cancelled) return;

        setProgramDeployed(!!programInfo?.executable);
        setSolBalance(lamports / LAMPORTS_PER_SOL);
        setVaultPda(pda);

        if (!vault) {
          setVaultExists(false);
          setSummary(null);
          setPendingRequests([]);
          return;
        }

        const [tokenBal, pending] = await Promise.all([
          provider.connection.getTokenAccountBalance(vault.vaultTokenAccount),
          // Only this vault's requests: `vault` is the first field after the 8-byte discriminator.
          program.account.pendingWithdrawal.all([{ memcmp: { offset: 8, bytes: pda.toBase58() } }]),
        ]);
        if (cancelled) return;

        setSummary({
          dailyLimit: vault.dailyLimit,
          spentToday: vault.spentToday,
          isPool: vault.isPool,
          vaultTokenAccount: vault.vaultTokenAccount,
          balance: new BN(tokenBal.value.amount),
        });
        setPendingRequests(pending.filter((req) => !req.account.executed));
        setVaultExists(true);
      } catch (e: unknown) {
        console.error('Error loading vault:', e);
        if (!cancelled) {
          toast.error('Could not reach Solana Devnet. Check your connection (or VPN) and refresh.');
        }
      }
    };

    load();
    return () => {
      cancelled = true;
    };
  }, [program, provider, refreshCounter]);

  const handleAirdrop = async () => {
    if (!provider) return;
    setAirdropping(true);
    const toastId = toast.loading('Requesting 1 Devnet SOL...');
    try {
      const sig = await provider.connection.requestAirdrop(provider.wallet.publicKey, LAMPORTS_PER_SOL);
      const latest = await provider.connection.getLatestBlockhash();
      await provider.connection.confirmTransaction({ signature: sig, ...latest }, 'confirmed');
      toast.success('Received 1 Devnet SOL', { id: toastId });
      refresh();
    } catch (e) {
      console.error(e);
      toast.error('Devnet faucet is rate-limited. Use faucet.solana.com to fund your wallet.', { id: toastId });
    } finally {
      setAirdropping(false);
    }
  };

  const handleInitializeVault = async () => {
    if (!program || !provider) return;

    if (programDeployed === false) {
      toast.error('The Trellis program is not deployed on Devnet yet.');
      return;
    }
    if (solBalance !== null && solBalance < MIN_SOL_FOR_INIT) {
      toast.error(`You need at least ${MIN_SOL_FOR_INIT} Devnet SOL to create a vault.`);
      return;
    }

    setInitializing(true);
    const toastId = toast.loading('Creating your vault on Devnet...');
    try {
      const owner = provider.wallet.publicKey;
      const [pda] = PublicKey.findProgramAddressSync(
        [Buffer.from('vault'), owner.toBuffer(), Buffer.from('personal')],
        program.programId
      );
      const [beneficiaryListPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('beneficiaries'), pda.toBuffer()],
        program.programId
      );
      const vaultTokenAccount = getAssociatedTokenAddressSync(USDC_MINT, pda, true);

      // Personal vault: the program ignores daily_limit and stores u64::MAX.
      const sig = await program.methods
        .initializeVault(false, new BN(0))
        .accountsPartial({
          payer: owner,
          owner,
          vault: pda,
          beneficiaryList: beneficiaryListPda,
          mint: USDC_MINT,
          vaultTokenAccount,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      toast.success('Vault created!', {
        id: toastId,
        action: { label: 'View', onClick: () => window.open(explorerTxUrl(sig), '_blank') },
      });
      refresh();
    } catch (e: unknown) {
      console.error(e);
      toast.error(e instanceof Error ? e.message : 'Initialization failed', { id: toastId });
    } finally {
      setInitializing(false);
    }
  };

  if (!connected) {
    return (
      <div className="flex flex-col items-center justify-center min-h-[60vh] gap-4">
        <h1 className="text-3xl font-bold text-white tracking-tight">Trellis Admin</h1>
        <p className="text-slate-400 font-light">Connect a wallet set to Solana Devnet to manage your vault.</p>
      </div>
    );
  }

  // Still checking existence
  if (vaultExists === null) {
    return (
      <div className="flex items-center justify-center min-h-[60vh]">
        <Loader2 className="w-8 h-8 animate-spin text-emerald-500" />
        <Toaster theme="dark" />
      </div>
    );
  }

  // Vault does not exist -> Show initialization screen
  if (vaultExists === false) {
    const lowSol = solBalance !== null && solBalance < MIN_SOL_FOR_INIT;
    return (
      <div className="flex flex-col items-center justify-center min-h-[60vh] gap-6 animate-in fade-in duration-500 max-w-lg mx-auto text-center">
        <h1 className="text-3xl md:text-4xl font-bold text-white tracking-tight mt-8">Welcome to Trellis</h1>
        <p className="text-slate-400 font-light leading-relaxed">
          Your secure, self-custodial on-chain inheritance and spending limit system. To get started, create your personal vault on Solana Devnet.
        </p>

        {programDeployed === false && (
          <p className="w-full rounded-xl border border-red-500/40 bg-red-500/10 p-4 text-sm text-red-300">
            The Trellis program is not deployed on Devnet yet.
          </p>
        )}

        {lowSol && (
          <div className="w-full rounded-xl border border-blue-500/30 bg-blue-950/20 p-6 text-sm text-blue-200 flex flex-col gap-4">
            <span className="leading-relaxed">
              Your wallet has {solBalance?.toFixed(4)} Devnet SOL. Creating a vault needs about {MIN_SOL_FOR_INIT} SOL.
              Make sure your wallet is set to <b>Devnet</b>, then fund it at{' '}
              <a href="https://faucet.solana.com/" target="_blank" rel="noopener noreferrer" className="underline hover:text-white transition-colors">
                faucet.solana.com
              </a>
              .
            </span>
            <Button
              id="airdrop-sol-button"
              variant="outline"
              onClick={handleAirdrop}
              disabled={airdropping}
              className="border-blue-500/50 text-blue-300 hover:bg-blue-500/20 bg-transparent rounded-xl"
            >
              {airdropping ? 'Requesting...' : 'Try 1 SOL airdrop'}
            </Button>
          </div>
        )}

        <Button
          id="initialize-vault-button"
          size="lg"
          onClick={handleInitializeVault}
          disabled={initializing || lowSol || programDeployed === false}
          className="bg-blue-600 hover:bg-blue-500 text-white w-full py-6 text-lg mt-6 shadow-[0_0_20px_-5px_rgba(37,99,235,0.5)] rounded-full transition-all hover:scale-105"
        >
          {initializing ? (
             <span className="flex items-center gap-2">
               <Loader2 className="w-5 h-5 animate-spin" /> Deploying Protocol...
             </span>
          ) : (
            "Deploy Trellis Vault"
          )}
        </Button>
        <Toaster theme="dark" />
      </div>
    );
  }

  const unlimited = summary ? summary.dailyLimit.eq(U64_MAX) : false;
  const spentPct =
    summary && !unlimited && !summary.dailyLimit.isZero()
      ? Math.min(100, summary.spentToday.muln(10000).div(summary.dailyLimit).toNumber() / 100)
      : 0;

  return (
    <div className="flex flex-col gap-10 max-w-6xl mx-auto p-6 animate-in fade-in duration-500 relative">
      <div className="absolute top-0 right-1/4 w-[500px] h-[500px] bg-blue-900/10 blur-[120px] pointer-events-none rounded-full"></div>
      <div className="absolute top-1/2 left-0 w-[400px] h-[400px] bg-emerald-900/10 blur-[100px] pointer-events-none rounded-full"></div>
      
      <div className="flex justify-between items-end relative z-10">
        <div>
          <h1 className="text-4xl md:text-5xl font-bold text-white tracking-tight">Vault Dashboard</h1>
          <p className="text-slate-400 font-light mt-3 text-lg">Manage beneficiaries and guardians for your vault.</p>
          {vaultPda && (
            <a
              href={explorerAddressUrl(vaultPda.toBase58())}
              target="_blank"
              rel="noopener noreferrer"
              className="text-xs font-mono text-slate-500 hover:text-emerald-400 transition-colors mt-2 inline-block tracking-widest"
            >
              [ VAULT: {vaultPda.toBase58().slice(0, 4)}…{vaultPda.toBase58().slice(-4)} ]
            </a>
          )}
        </div>

        {/* Alerts Section */}
        <div className="flex flex-col items-end gap-2 relative z-10">
          {pendingRequests.length > 0 ? (
            <Button
              variant="outline"
              className="border-yellow-500/50 text-yellow-500 hover:bg-transparent cursor-default gap-2 shadow-[0_0_15px_rgba(234,179,8,0.15)] rounded-xl"
              title="Waiting for a guardian to approve from their own device. The vault owner cannot approve their own withdrawals."
            >
              <span className="h-2 w-2 rounded-full bg-yellow-500 animate-pulse" />
              {pendingRequests.length} Awaiting Guardian{pendingRequests.length > 1 ? 's' : ''}
            </Button>
          ) : (
            <Button
              variant="outline"
              className="border-white/10 text-slate-500 bg-white/[0.02] gap-2 cursor-default rounded-xl"
            >
              <span className="h-2 w-2 rounded-full bg-slate-700" />
              No Pending Requests
            </Button>
          )}
        </div>
      </div>

      {/* Financial Overview */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 relative z-10">
        <div className="p-8 rounded-3xl border border-white/10 bg-white/[0.02] backdrop-blur-xl shadow-lg flex flex-col justify-between">
          <div>
            <h3 className="text-xs font-mono font-bold text-slate-500 uppercase tracking-widest">Total Balance</h3>
            <p className="text-4xl md:text-5xl font-mono tracking-tight text-white mt-4">{summary ? formatUsdc(summary.balance) : '—'}</p>
          </div>
          <div className="flex items-center justify-between mt-8 pt-6 border-t border-white/5">
            <span className="text-xs font-mono text-slate-500 uppercase">USDC Devnet</span>
            <Button
              id="open-deposit-button"
              size="sm"
              variant="outline"
              onClick={() => setIsDepositOpen(true)}
              className="h-8 rounded-xl border-emerald-500/40 text-emerald-400 hover:bg-emerald-500/10"
            >
              Deposit
            </Button>
          </div>
        </div>
        <div className="p-8 rounded-3xl border border-white/10 bg-white/[0.02] backdrop-blur-xl shadow-lg flex flex-col justify-between">
          <div>
            <h3 className="text-xs font-mono font-bold text-slate-500 uppercase tracking-widest">Daily Limit</h3>
            <p className="text-4xl md:text-5xl font-mono tracking-tight text-white mt-4">
              {summary ? (unlimited ? 'Unlimited' : formatUsdc(summary.dailyLimit)) : '—'}
            </p>
          </div>
          <span className="text-xs font-mono text-slate-500 mt-8 pt-6 border-t border-white/5 block leading-relaxed">
            {summary?.isPool ? 'Max auto-approved per day' : 'Personal vault — owner withdrawals are not capped'}
          </span>
        </div>
        <div className="p-8 rounded-3xl border border-white/10 bg-white/[0.02] backdrop-blur-xl shadow-lg relative overflow-hidden flex flex-col justify-between">
          <div className="absolute top-0 right-0 p-6 opacity-[0.03]">
            <svg width="60" height="60" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12V7H5a2 2 0 0 1 0-4h14v4"/><path d="M3 5v14a2 2 0 0 0 2 2h16v-5"/><path d="M18 12a2 2 0 0 0 0 4h4v-4Z"/></svg>
          </div>
          <div>
            <h3 className="text-xs font-mono font-bold text-slate-500 uppercase tracking-widest">Spent Today</h3>
            <p className="text-4xl md:text-5xl font-mono tracking-tight text-emerald-400 mt-4">{summary ? formatUsdc(summary.spentToday) : '—'}</p>
          </div>
          <div className="mt-8 pt-6 border-t border-white/5">
            <div className="w-full bg-black/50 rounded-full h-2 shadow-inner">
              <div className="bg-emerald-500 h-2 rounded-full shadow-[0_0_10px_rgba(16,185,129,0.5)]" style={{ width: `${spentPct}%` }}></div>
            </div>
          </div>
        </div>
      </div>

      {/* Beneficiary Management */}
      <div className="relative z-10">
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-2xl font-bold text-white tracking-tight">Approved Beneficiaries</h2>
          <div className="flex gap-2">
            <Button
              id="open-add-guardian-button"
              size="sm"
              variant="outline"
              onClick={() => setIsApproverModalOpen(true)}
              className="rounded-xl border-white/10 text-slate-300 hover:bg-white/5 hover:text-white transition-colors"
            >
              + Add Guardian
            </Button>
            <AddBeneficiaryModal onAdded={refresh} />
          </div>
        </div>
        {vaultPda && <BeneficiaryTable vaultPda={vaultPda} refreshCounter={refreshCounter} />}
      </div>

      {vaultPda && (
        <AddApproverModal
          isOpen={isApproverModalOpen}
          onClose={() => setIsApproverModalOpen(false)}
          vaultPda={vaultPda}
          onSuccess={refresh}
        />
      )}
      {vaultPda && summary && (
        <DepositUsdcModal
          isOpen={isDepositOpen}
          onClose={() => setIsDepositOpen(false)}
          vaultPda={vaultPda}
          vaultTokenAccount={summary.vaultTokenAccount}
          onSuccess={refresh}
        />
      )}
      <Toaster theme="dark" />
    </div>
  );
}


