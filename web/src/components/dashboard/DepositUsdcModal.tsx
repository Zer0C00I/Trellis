import { useEffect, useState } from 'react'
import { PublicKey } from '@solana/web3.js'
import { BN } from '@coral-xyz/anchor'
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from '@solana/spl-token'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '../ui/dialog'
import { Button } from '../ui/button'
import { Input } from '../ui/input'
import { Label } from '../ui/label'
import { useAnchorProvider } from '../../config/useAnchorProvider'
import { USDC_MINT, USDC_DECIMALS, explorerTxUrl } from '../../config/solana'
import { toast } from 'sonner'
import { Loader2 } from 'lucide-react'

interface DepositUsdcModalProps {
  isOpen: boolean
  onClose: () => void
  vaultPda: PublicKey
  vaultTokenAccount: PublicKey
  onSuccess?: () => void
}

/** Parses a decimal USDC string into base units without floating point rounding. */
function toBaseUnits(value: string): BN | null {
  const trimmed = value.trim()
  if (!/^\d+(\.\d+)?$/.test(trimmed)) return null
  const [whole, frac = ''] = trimmed.split('.')
  if (frac.length > USDC_DECIMALS) return null
  const units = new BN(whole + frac.padEnd(USDC_DECIMALS, '0'))
  return units.isZero() ? null : units
}

export function DepositUsdcModal({ isOpen, onClose, vaultPda, vaultTokenAccount, onSuccess }: DepositUsdcModalProps) {
  const { program, provider } = useAnchorProvider()
  const [amount, setAmount] = useState('')
  const [isSubmitting, setIsSubmitting] = useState(false)
  // The depositor's USDC, shown for reference and for "Max". null = unknown, '' = no token account.
  const [walletUsdc, setWalletUsdc] = useState<string | null>(null)

  useEffect(() => {
    if (!isOpen || !provider) return
    let cancelled = false
    const ata = getAssociatedTokenAddressSync(USDC_MINT, provider.wallet.publicKey)
    provider.connection
      .getTokenAccountBalance(ata)
      .then((b) => !cancelled && setWalletUsdc(b.value.uiAmountString ?? '0'))
      .catch(() => !cancelled && setWalletUsdc(''))
    return () => {
      cancelled = true
    }
  }, [isOpen, provider])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!program || !provider) return

    const units = toBaseUnits(amount)
    if (!units) {
      toast.error(`Enter a valid USDC amount (max ${USDC_DECIMALS} decimals).`)
      return
    }

    const depositor = provider.wallet.publicKey
    const depositorTokenAccount = getAssociatedTokenAddressSync(USDC_MINT, depositor)

    setIsSubmitting(true)
    const toastId = toast.loading('Depositing USDC…')
    try {
      const info = await provider.connection.getAccountInfo(depositorTokenAccount)
      if (!info) {
        toast.error('You have no Devnet USDC token account. Get test USDC at faucet.circle.com (Solana Devnet).', {
          id: toastId,
        })
        return
      }
      const bal = await provider.connection.getTokenAccountBalance(depositorTokenAccount)
      if (new BN(bal.value.amount).lt(units)) {
        toast.error(`Insufficient USDC. Wallet balance: ${bal.value.uiAmountString} USDC`, { id: toastId })
        return
      }

      const sig = await program.methods
        .deposit(units)
        .accountsPartial({
          depositor,
          depositorTokenAccount,
          vaultTokenAccount,
          vault: vaultPda,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc()

      toast.success('Deposit confirmed', {
        id: toastId,
        action: { label: 'View', onClick: () => window.open(explorerTxUrl(sig), '_blank') },
      })
      setAmount('')
      onSuccess?.()
      onClose()
    } catch (error: unknown) {
      console.error(error)
      toast.error(error instanceof Error ? error.message : 'Deposit failed', { id: toastId })
    } finally {
      setIsSubmitting(false)
    }
  }

  const trimmed = amount.trim()
  const amountInvalid = trimmed.length > 0 && !toBaseUnits(trimmed)

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[460px]">
        <DialogHeader>
          <DialogTitle>Deposit USDC</DialogTitle>
          <DialogDescription>
            Moves Devnet USDC from your wallet into the vault. Need test USDC? Get some from the{' '}
            <a
              href="https://faucet.circle.com/"
              target="_blank"
              rel="noopener noreferrer"
              className="text-wisteria underline decoration-wisteria/40 underline-offset-4 hover:decoration-wisteria"
            >
              Circle faucet
            </a>{' '}
            (pick Solana Devnet).
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-5" noValidate>
          <div className="space-y-2">
            <div className="flex items-baseline justify-between gap-3">
              <Label htmlFor="deposit-usdc-amount">Amount</Label>
              <span className="text-sm text-moss">
                {walletUsdc === null
                  ? provider
                    ? 'Checking your balance…'
                    : null
                  : walletUsdc === ''
                    ? 'No USDC in this wallet yet'
                    : `In your wallet: ${walletUsdc} USDC`}
              </span>
            </div>
            <div className="relative">
              <Input
                id="deposit-usdc-amount"
                inputMode="decimal"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                placeholder="0.00"
                className="tabular h-14 pr-28 text-2xl font-semibold"
                autoComplete="off"
                required
                aria-invalid={amountInvalid || undefined}
                aria-describedby="deposit-hint"
              />
              <span
                className={`pointer-events-none absolute top-1/2 -translate-y-1/2 text-[15px] text-moss ${walletUsdc && walletUsdc !== '0' ? 'right-16' : 'right-4'}`}
              >
                USDC
              </span>
              {walletUsdc && walletUsdc !== '0' && (
                <button
                  type="button"
                  onClick={() => setAmount(walletUsdc)}
                  className="absolute top-1/2 right-2 h-9 -translate-y-1/2 rounded-md px-2.5 text-sm font-medium text-wisteria hover:bg-bloom/60"
                >
                  Max
                </button>
              )}
            </div>
            <p id="deposit-hint" className={`text-sm ${amountInvalid ? 'text-rust' : 'text-moss'}`}>
              {amountInvalid
                ? `Enter a number above zero, with up to ${USDC_DECIMALS} decimals.`
                : 'You can deposit any amount. The daily limit only applies to sending.'}
            </p>
          </div>
          <DialogFooter className="gap-2 sm:gap-2">
            <Button type="button" variant="ghost" onClick={onClose} disabled={isSubmitting}>
              Cancel
            </Button>
            <Button id="deposit-usdc-submit" type="submit" disabled={isSubmitting || !toBaseUnits(trimmed)}>
              {isSubmitting ? (
                <>
                  <Loader2 className="animate-spin" /> Depositing
                </>
              ) : (
                'Deposit'
              )}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
