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
      toastId = toast.loading('Adding trusted guardian...')

      const owner = provider.wallet.publicKey

      const [approversPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('approvers'), vaultPda.toBuffer()],
        program.programId,
      )

      // We need to check if the approvers PDA exists first.
      const approversAccount = await program.account.trustedApprovers.fetchNullable(approversPda)

      if (!approversAccount) {
        // Initialize approvers system first
        toast.loading('Initializing Guardian System...', { id: toastId })
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

      toast.loading('Adding Guardian...', { id: toastId })

      const tx = await program.methods
        .addTrustedApprover(pubkey)
        .accountsPartial({
          signer: owner,
          vault: vaultPda,
          approvers: approversPda,
          existingApprover: null, // First approver doesn't need an existing co-signer
        })
        .rpc()

      toast.success('Guardian added! Tx: ' + tx.slice(0, 8) + '...', { id: toastId })
      setAddress('')
      if (onSuccess) onSuccess()
      onClose()
    } catch (error: unknown) {
      console.error(error)
      const message = error instanceof Error ? error.message : String(error)
      const friendly = message.includes('ExistingApproverRequired')
        ? 'Adding a second guardian requires an existing guardian to co-sign.'
        : message.includes('CannotApproveOwnWithdrawals')
          ? 'You cannot add your own wallet as a guardian.'
          : message || 'Failed to add guardian'
      toast.error(friendly, { id: toastId })
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[425px] bg-zinc-950 border border-zinc-800 text-zinc-100">
        <DialogHeader>
          <DialogTitle>Add Trusted Guardian</DialogTitle>
          <DialogDescription className="text-zinc-400">
            Guardians can co-sign large withdrawal requests. They cannot initiate withdrawals themselves.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="grid gap-4 py-4">
          <div className="grid gap-2">
            <label className="text-sm font-medium text-zinc-400">Guardian Solana Address</label>
            <Input
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              placeholder="Enter public key..."
              className="bg-zinc-900 border-zinc-800"
              required
            />
          </div>
          <DialogFooter className="mt-4">
            <Button
              type="button"
              variant="ghost"
              onClick={onClose}
              disabled={isSubmitting}
              className="hover:bg-zinc-800 text-zinc-400"
            >
              Cancel
            </Button>
            <Button type="submit" disabled={isSubmitting} className="bg-emerald-600 hover:bg-emerald-700 text-white">
              {isSubmitting ? (
                <span className="flex items-center gap-2">
                  <Loader2 className="w-4 h-4 animate-spin" /> Adding...
                </span>
              ) : (
                'Add Guardian'
              )}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
