import { useState } from 'react'
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
    const toastId = toast.loading('Depositing USDC...')
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

      toast.success('Deposit confirmed!', {
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

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[425px] bg-zinc-950 border border-zinc-800 text-zinc-100">
        <DialogHeader>
          <DialogTitle>Deposit USDC</DialogTitle>
          <DialogDescription className="text-zinc-400">
            Move Devnet USDC from your wallet into your vault. Need test USDC? Use the{' '}
            <a href="https://faucet.circle.com/" target="_blank" rel="noopener noreferrer" className="underline">
              Circle faucet
            </a>{' '}
            (Solana Devnet).
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="grid gap-4 py-4">
          <div className="grid gap-2">
            <Label htmlFor="deposit-usdc-amount" className="text-zinc-400">
              Amount (USDC)
            </Label>
            <Input
              id="deposit-usdc-amount"
              inputMode="decimal"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              placeholder="0.00"
              className="bg-zinc-900 border-zinc-800 font-mono"
              required
            />
          </div>
          <DialogFooter className="mt-2">
            <Button
              type="button"
              variant="ghost"
              onClick={onClose}
              disabled={isSubmitting}
              className="hover:bg-zinc-800 text-zinc-400"
            >
              Cancel
            </Button>
            <Button
              id="deposit-usdc-submit"
              type="submit"
              disabled={isSubmitting}
              className="bg-emerald-600 hover:bg-emerald-700 text-white"
            >
              {isSubmitting ? (
                <span className="flex items-center gap-2">
                  <Loader2 className="w-4 h-4 animate-spin" /> Depositing...
                </span>
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
