import { useState } from 'react'
import { PublicKey, SystemProgram } from '@solana/web3.js'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '../ui/dialog'
import { Button } from '../ui/button'
import { Input } from '../ui/input'
import { useAnchorProvider } from '../../config/useAnchorProvider'
import { toast } from 'sonner'
import { Loader2 } from 'lucide-react'

interface AddApproverModalProps {
  isOpen: boolean
  onClose: () => void
  vaultPda: PublicKey
  onSuccess?: () => void
}

export function AddApproverModal({ isOpen, onClose, vaultPda, onSuccess }: AddApproverModalProps) {
  const { program, provider } = useAnchorProvider()
  const [address, setAddress] = useState('')
  const [isSubmitting, setIsSubmitting] = useState(false)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!program || !provider || !provider.wallet) return

    let toastId: string | number | undefined
    try {
      const pubkey = new PublicKey(address.trim())
      setIsSubmitting(true)
      toastId = toast.loading('Adding guardian…')

      const owner = provider.wallet.publicKey

      const [approversPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('approvers'), vaultPda.toBuffer()],
        program.programId,
      )

      const approversAccount = await program.account.trustedApprovers.fetchNullable(approversPda)

      if (!approversAccount) {
        toast.loading('Setting up guardians for this vault…', { id: toastId })
        await program.methods
          .initializeApprovers()
          .accountsPartial({
            payer: owner,
            vault: vaultPda,
            approvers: approversPda,
            systemProgram: SystemProgram.programId,
          })
          .rpc()
      }

      toast.loading('Adding guardian…', { id: toastId })

      const tx = await program.methods
        .addTrustedApprover(pubkey)
        .accountsPartial({
          signer: owner,
          vault: vaultPda,
          approvers: approversPda,
          existingApprover: null, // First approver doesn't need an existing co-signer
        })
        .rpc()

      toast.success('Guardian added', { id: toastId, description: `Transaction ${tx.slice(0, 8)}…` })
      setAddress('')
      if (onSuccess) onSuccess()
      onClose()
    } catch (error: unknown) {
      console.error(error)
      const message = error instanceof Error ? error.message : String(error)
      const friendly = message.includes('ExistingApproverRequired')
        ? 'Adding a second guardian requires an existing guardian to co-sign.'
        : message.includes('CannotApproveOwnWithdrawals')
          ? "Your own wallet can't be a guardian. Use someone else's address."
          : message || 'Could not add the guardian.'
      toast.error(friendly, { id: toastId })
    } finally {
      setIsSubmitting(false)
    }
  }

  const trimmed = address.trim()
  let valid = false
  try {
    valid = trimmed.length >= 32 && !!new PublicKey(trimmed)
  } catch {
    valid = false
  }
  const ownAddress = valid && provider?.wallet.publicKey.toBase58() === trimmed
  const invalid = trimmed.length > 0 && (!valid || ownAddress)

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[460px]">
        <DialogHeader>
          <DialogTitle>Add a guardian</DialogTitle>
          <DialogDescription>
            A guardian signs requests above the daily limit. They can&apos;t start a withdrawal themselves. Adding a
            second guardian needs the first one to co-sign.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-5" noValidate>
          <div className="space-y-2">
            <label htmlFor="guardian-address" className="block text-[15px] font-medium text-ivy">
              Guardian&apos;s Solana address
            </label>
            <Input
              id="guardian-address"
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              placeholder="e.g. 9ozR…4zwJ"
              className="font-mono text-sm"
              autoComplete="off"
              spellCheck={false}
              required
              aria-invalid={invalid || undefined}
              aria-describedby="guardian-hint"
            />
            <p id="guardian-hint" className={`text-sm ${invalid ? 'text-rust' : 'text-moss'}`}>
              {ownAddress
                ? "That's your own wallet. A guardian has to be someone else."
                : invalid
                  ? "That isn't a Solana address. Check for missing characters."
                  : 'Pick someone you trust who answers their phone.'}
            </p>
          </div>
          <DialogFooter className="gap-2 sm:gap-2">
            <Button type="button" variant="ghost" onClick={onClose} disabled={isSubmitting}>
              Cancel
            </Button>
            <Button type="submit" disabled={isSubmitting || !valid || ownAddress}>
              {isSubmitting ? (
                <>
                  <Loader2 className="animate-spin" /> Adding
                </>
              ) : (
                'Add guardian'
              )}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
