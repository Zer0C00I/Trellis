'use client'
import { useState, useEffect, useCallback, useMemo } from 'react'
import { useWallet } from '@solana/wallet-adapter-react'
import { useWalletModal } from '@solana/wallet-adapter-react-ui'
import { LAMPORTS_PER_SOL, PublicKey, SystemProgram } from '@solana/web3.js'
import { BN, ProgramAccount } from '@coral-xyz/anchor'
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID } from '@solana/spl-token'
import { BeneficiaryTable, type Beneficiary } from '../../components/dashboard/BeneficiaryTable'
import { AddBeneficiaryModal } from '../../components/dashboard/AddBeneficiaryModal'
import { AddApproverModal } from '../../components/dashboard/AddApproverModal'
import { DepositUsdcModal } from '../../components/dashboard/DepositUsdcModal'
import { VaultPanel } from '../../components/dashboard/VaultPanel'
import { GuardianList } from '../../components/dashboard/GuardianList'
import { PendingList } from '../../components/dashboard/PendingList'
import { SetupChecklist } from '../../components/dashboard/SetupChecklist'
import { Button } from '../../components/ui/button'
import { toast } from 'sonner'
import { useAnchorProvider } from '../../config/useAnchorProvider'
import {
  USDC_MINT,
  USDC_DECIMALS,
  DEFAULT_POOL_DAILY_LIMIT_USDC,
  findPoolVaultPda,
  explorerTxUrl,
} from '../../config/solana'
import { Check, Loader2, Plus } from 'lucide-react'

// Rent for vault + beneficiary list + vault token account is ~0.011 SOL; keep headroom for fees.
const MIN_SOL_FOR_INIT = 0.02

interface VaultSummary {
  dailyLimit: BN
  spentToday: BN
  isPool: boolean
  lastResetDay: number
  vaultTokenAccount: PublicKey
  balance: BN
}

export default function Dashboard() {
  const { connected } = useWallet()
  const { setVisible: openWalletModal } = useWalletModal()
  const { program, provider } = useAnchorProvider()

  const [isApproverModalOpen, setIsApproverModalOpen] = useState(false)
  const [isDepositOpen, setIsDepositOpen] = useState(false)
  const [isBeneficiaryOpen, setIsBeneficiaryOpen] = useState(false)
  const [refreshCounter, setRefreshCounter] = useState(0)
  const refresh = useCallback(() => setRefreshCounter((c) => c + 1), [])

  const [vaultExists, setVaultExists] = useState<boolean | null>(null)
  const [vaultPda, setVaultPda] = useState<PublicKey | null>(null)
  const [summary, setSummary] = useState<VaultSummary | null>(null)
  const [initializing, setInitializing] = useState(false)
  const [airdropping, setAirdropping] = useState(false)
  const [solBalance, setSolBalance] = useState<number | null>(null)
  const [programDeployed, setProgramDeployed] = useState<boolean | null>(null)

  const [pendingRequests, setPendingRequests] = useState<ProgramAccount[]>([])
  const [nowSecs, setNowSecs] = useState(0)
  const [family, setFamily] = useState<Beneficiary[] | null>(null)
  const [guardianCount, setGuardianCount] = useState<number | null>(null)

  useEffect(() => {
    if (!program || !provider) return
    let cancelled = false

    const load = async () => {
      try {
        const owner = provider.wallet.publicKey
        const pda = findPoolVaultPda(owner, program.programId)

        const [programInfo, lamports, vault] = await Promise.all([
          provider.connection.getAccountInfo(program.programId),
          provider.connection.getBalance(owner),
          program.account.vault.fetchNullable(pda),
        ])
        if (cancelled) return

        setProgramDeployed(!!programInfo?.executable)
        setSolBalance(lamports / LAMPORTS_PER_SOL)
        setVaultPda(pda)

        if (!vault) {
          setVaultExists(false)
          setSummary(null)
          setPendingRequests([])
          return
        }

        const [tokenBal, pending] = await Promise.all([
          provider.connection.getTokenAccountBalance(vault.vaultTokenAccount),
          // Only this vault's requests: `vault` is the first field after the 8-byte discriminator.
          program.account.pendingWithdrawal.all([{ memcmp: { offset: 8, bytes: pda.toBase58() } }]),
        ])
        if (cancelled) return

        setSummary({
          dailyLimit: vault.dailyLimit,
          spentToday: vault.spentToday,
          isPool: vault.isPool,
          lastResetDay: vault.lastResetDay.toNumber(),
          vaultTokenAccount: vault.vaultTokenAccount,
          balance: new BN(tokenBal.value.amount),
        })
        setPendingRequests(pending.filter((req) => !req.account.executed))
        setNowSecs(Math.floor(Date.now() / 1000))
        setVaultExists(true)
      } catch (e: unknown) {
        console.error('Error loading vault:', e)
        if (!cancelled) {
          toast.error('Could not reach Solana Devnet. Check your connection (or VPN) and refresh.')
        }
      }
    }

    load()
    return () => {
      cancelled = true
    }
  }, [program, provider, refreshCounter])

  const handleAirdrop = async () => {
    if (!provider) return
    setAirdropping(true)
    const toastId = toast.loading('Requesting 1 Devnet SOL...')
    try {
      const sig = await provider.connection.requestAirdrop(provider.wallet.publicKey, LAMPORTS_PER_SOL)
      const latest = await provider.connection.getLatestBlockhash()
      await provider.connection.confirmTransaction({ signature: sig, ...latest }, 'confirmed')
      toast.success('Received 1 Devnet SOL', { id: toastId })
      refresh()
    } catch (e) {
      console.error(e)
      toast.error('Devnet faucet is rate-limited. Use faucet.solana.com to fund your wallet.', { id: toastId })
    } finally {
      setAirdropping(false)
    }
  }

  const handleInitializeVault = async () => {
    if (!program || !provider) return

    if (programDeployed === false) {
      toast.error('The Trellis program is not deployed on Devnet yet.')
      return
    }
    if (solBalance !== null && solBalance < MIN_SOL_FOR_INIT) {
      toast.error(`You need at least ${MIN_SOL_FOR_INIT} Devnet SOL to create a vault.`)
      return
    }

    setInitializing(true)
    const toastId = toast.loading('Creating your vault on Devnet...')
    try {
      const owner = provider.wallet.publicKey
      const pda = findPoolVaultPda(owner, program.programId)
      const [beneficiaryListPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('beneficiaries'), pda.toBuffer()],
        program.programId,
      )
      const vaultTokenAccount = getAssociatedTokenAddressSync(USDC_MINT, pda, true)

      // Pool vault: daily_limit is in USDC base units and must be > 0.
      const dailyLimit = new BN(DEFAULT_POOL_DAILY_LIMIT_USDC).mul(new BN(10).pow(new BN(USDC_DECIMALS)))
      const sig = await program.methods
        .initializeVault(true, dailyLimit)
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
        .rpc()

      toast.success('Vault created!', {
        id: toastId,
        action: { label: 'View', onClick: () => window.open(explorerTxUrl(sig), '_blank') },
      })
      refresh()
    } catch (e: unknown) {
      console.error(e)
      toast.error(e instanceof Error ? e.message : 'Initialization failed', { id: toastId })
    } finally {
      setInitializing(false)
    }
  }

  const familyNames = useMemo(
    () => new Map((family ?? []).map((b) => [b.address.toBase58(), b.name] as [string, string])),
    [family],
  )

  if (!connected) {
    return (
      <div className="mx-auto max-w-xl py-16 sm:py-24">
        <h1 className="text-[34px] font-semibold leading-tight sm:text-[42px]">Your family vault</h1>
        <p className="mt-4 text-lg leading-relaxed text-moss">
          Connect a wallet set to Solana Devnet. Trellis reads your vault from the chain and asks your wallet to sign
          anything that changes it. It never holds your keys.
        </p>
        <Button size="lg" variant="secondary" className="mt-8" onClick={() => openWalletModal(true)}>
          Connect wallet
        </Button>
        <p className="mt-4 text-sm text-moss">
          No Phantom yet? Install it from phantom.com, then switch it to Devnet in Settings, Developer settings.
        </p>
      </div>
    )
  }

  if (vaultExists === null) {
    return (
      <div className="grid gap-6 py-8 lg:grid-cols-[1.15fr_1fr]" aria-busy="true" aria-label="Loading your vault">
        <div className="h-[340px] animate-pulse rounded-[14px] bg-ivy/80" />
        <div className="h-[340px] animate-pulse rounded-[14px] bg-ivy/[0.05]" />
      </div>
    )
  }

  if (vaultExists === false) {
    const lowSol = solBalance !== null && solBalance < MIN_SOL_FOR_INIT
    const notDeployed = programDeployed === false
    return (
      <div className="mx-auto max-w-xl py-12 sm:py-20">
        <h1 className="text-[34px] font-semibold leading-tight sm:text-[42px]">Create your vault</h1>
        <p className="mt-4 text-lg leading-relaxed text-moss">
          The vault holds USDC that only you can move, only to people you name, and no more than{' '}
          {DEFAULT_POOL_DAILY_LIMIT_USDC} USDC a day without a guardian.
        </p>

        <ol className="mt-10 border-t border-rule">
          <li className="flex gap-4 border-b border-rule py-5">
            <StepMark done />
            <div>
              <p className="text-[17px] font-medium">Wallet connected</p>
              <p className="mt-1 text-[15px] text-moss">Make sure it&apos;s set to Devnet.</p>
            </div>
          </li>
          <li className="flex gap-4 border-b border-rule py-5">
            <StepMark done={solBalance !== null && !lowSol} />
            <div className="flex-1">
              <p className="text-[17px] font-medium">Devnet SOL for rent and fees</p>
              <p className="mt-1 text-[15px] leading-relaxed text-moss">
                You have <span className="tabular">{solBalance?.toFixed(4) ?? '…'}</span> SOL. Creating a vault needs
                about {MIN_SOL_FOR_INIT} SOL.
              </p>
              {lowSol && (
                <div className="mt-3 flex flex-col gap-3 sm:flex-row sm:items-center">
                  <Button id="airdrop-sol-button" variant="outline" onClick={handleAirdrop} disabled={airdropping}>
                    {airdropping ? (
                      <>
                        <Loader2 className="animate-spin" /> Requesting 1 SOL
                      </>
                    ) : (
                      'Request 1 test SOL'
                    )}
                  </Button>
                  <a
                    href="https://faucet.solana.com/"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-[15px] text-wisteria underline decoration-wisteria/40 underline-offset-4 hover:decoration-wisteria"
                  >
                    Or use faucet.solana.com
                  </a>
                </div>
              )}
            </div>
          </li>
          <li className="flex gap-4 border-b border-rule py-5">
            <StepMark done={false} />
            <div className="flex-1">
              <p className="text-[17px] font-medium">Create the vault</p>
              <p className="mt-1 text-[15px] text-moss">One transaction. Your wallet will ask you to approve it.</p>
              {notDeployed && (
                <p role="alert" className="mt-3 border-l-4 border-rust bg-rust/[0.07] px-4 py-3 text-[15px] text-rust">
                  The Trellis program isn&apos;t deployed on Devnet right now, so vaults can&apos;t be created.
                </p>
              )}
              <Button
                id="initialize-vault-button"
                size="lg"
                onClick={handleInitializeVault}
                disabled={initializing || lowSol || notDeployed}
                className="mt-4 w-full sm:w-auto"
              >
                {initializing ? (
                  <>
                    <Loader2 className="animate-spin" /> Creating vault
                  </>
                ) : (
                  'Create vault'
                )}
              </Button>
            </div>
          </li>
        </ol>
      </div>
    )
  }

  // spent_today only resets on-chain at the next withdrawal of a new UTC day,
  // so a stale counter from an earlier day means nothing has been sent today.
  const today = Math.floor(nowSecs / 86_400)
  const spentToday = summary && summary.lastResetDay === today ? summary.spentToday : new BN(0)

  return (
    <div className="py-6 sm:py-10">
      <h1 className="sr-only">Vault dashboard</h1>

      <div className="grid gap-6 lg:grid-cols-[1.15fr_1fr] lg:gap-8">
        {summary && vaultPda ? (
          <VaultPanel
            balance={summary.balance}
            dailyLimit={summary.dailyLimit}
            spentToday={spentToday}
            isPool={summary.isPool}
            vaultAddress={vaultPda.toBase58()}
            onDeposit={() => setIsDepositOpen(true)}
          />
        ) : (
          <div className="h-[340px] animate-pulse rounded-[14px] bg-ivy/80" />
        )}

        <div className="flex flex-col gap-6">
          <SetupChecklist
            steps={[
              { label: 'Create the vault', done: true },
              {
                label: 'Deposit some USDC',
                done: !!summary && !summary.balance.isZero(),
                action: { label: 'Deposit', onClick: () => setIsDepositOpen(true) },
              },
              {
                label: 'Add a family member',
                done: (family?.length ?? 0) > 0,
                action: { label: 'Add', onClick: () => setIsBeneficiaryOpen(true) },
              },
              {
                label: 'Add a guardian',
                done: (guardianCount ?? 0) > 0,
                action: { label: 'Add', onClick: () => setIsApproverModalOpen(true) },
              },
            ]}
          />
          <PendingList requests={pendingRequests} names={familyNames} nowSecs={nowSecs} />
        </div>
      </div>

      <section aria-labelledby="family-heading" className="mt-12">
        <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
          <div>
            <h2 id="family-heading" className="text-[27px] font-semibold">
              Family
            </h2>
            <p className="mt-1 text-[15px] text-moss">The only people this vault can pay.</p>
          </div>
          <AddBeneficiaryModal onAdded={refresh} open={isBeneficiaryOpen} onOpenChange={setIsBeneficiaryOpen} />
        </div>
        {vaultPda && <BeneficiaryTable vaultPda={vaultPda} refreshCounter={refreshCounter} onLoaded={setFamily} />}
      </section>

      <section aria-labelledby="guardians-heading" className="mt-12">
        <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
          <div>
            <h2 id="guardians-heading" className="text-[27px] font-semibold">
              Guardians
            </h2>
            <p className="mt-1 text-[15px] text-moss">
              They sign anything above the daily limit, from their own device.
            </p>
          </div>
          <Button id="open-add-guardian-button" variant="outline" onClick={() => setIsApproverModalOpen(true)}>
            <Plus /> Add guardian
          </Button>
        </div>
        {vaultPda && <GuardianList vaultPda={vaultPda} refreshCounter={refreshCounter} onLoaded={setGuardianCount} />}
      </section>

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
    </div>
  )
}

function StepMark({ done }: { done: boolean }) {
  return (
    <span
      aria-label={done ? 'Done' : 'To do'}
      className={`mt-0.5 grid size-7 shrink-0 place-items-center rounded-full border-2 ${
        done ? 'border-sap bg-sap text-white' : 'border-ivy/30'
      }`}
    >
      {done && <Check className="size-4" strokeWidth={3} />}
    </span>
  )
}
