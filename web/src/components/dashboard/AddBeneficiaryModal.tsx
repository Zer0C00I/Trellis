import { useState } from 'react'
import { PublicKey } from '@solana/web3.js'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from '../ui/dialog'
import { Button } from '../ui/button'
import { Input } from '../ui/input'
import { Label } from '../ui/label'
import { useAnchorProvider } from '../../config/useAnchorProvider'
import { findPoolVaultPda } from '../../config/solana'
import { toast } from 'sonner'
import { Loader2, Plus } from 'lucide-react'

function isSolanaAddress(value: string) {
  try {
    new PublicKey(value.trim())
    return value.trim().length >= 32
  } catch {
    return false
  }
}

export function AddBeneficiaryModal({
  onAdded,
  open: openProp,
  onOpenChange,
}: {
  onAdded: () => void
  open?: boolean
  onOpenChange?: (open: boolean) => void
}) {
  const { program, provider } = useAnchorProvider()
  const [innerOpen, setInnerOpen] = useState(false)
  const open = openProp ?? innerOpen
  const setOpen = onOpenChange ?? setInnerOpen
  const [loading, setLoading] = useState(false)
  const [name, setName] = useState('')
  const [address, setAddress] = useState('')

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!program || !provider || !provider.wallet) return

    try {
      setLoading(true)

      let targetAddress: PublicKey
      try {
        targetAddress = new PublicKey(address)
      } catch {
        toast.error('Invalid Solana address.')
        return
      }

      const nameBytes = new TextEncoder().encode(name.trim()).length
      if (nameBytes === 0 || nameBytes > 32) {
        toast.error('Name must be 1-32 bytes long.')
        return
      }

      const owner = provider.wallet.publicKey

      const vaultPda = findPoolVaultPda(owner, program.programId)

      const [beneficiaryListPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('beneficiaries'), vaultPda.toBuffer()],
        program.programId,
      )

      const tx = await program.methods
        .addBeneficiary(name.trim(), targetAddress)
        .accountsPartial({
          signer: owner,
          vault: vaultPda,
          beneficiaryList: beneficiaryListPda,
        })
        .rpc()

      toast.success(`Family member added. They can be paid in 48 hours.`, {
        description: `Transaction ${tx.slice(0, 8)}…`,
      })
      setOpen(false)
      setName('')
      setAddress('')
      onAdded()
    } catch (err: unknown) {
      console.error(err)
      const message = err instanceof Error ? err.message : String(err)
      if (message.includes('AccountNotInitialized')) {
        toast.error('Create your vault first, then add family members.')
      } else {
        toast.error(message || 'Could not add the family member.')
      }
    } finally {
      setLoading(false)
    }
  }

  const nameBytes = new TextEncoder().encode(name.trim()).length
  const nameTooLong = nameBytes > 32
  const addressInvalid = address.trim().length > 0 && !isSolanaAddress(address)
  const canSubmit = nameBytes > 0 && !nameTooLong && isSolanaAddress(address) && !loading

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>
          <Plus /> Add family member
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-[460px]">
        <DialogHeader>
          <DialogTitle>Add a family member</DialogTitle>
          <DialogDescription>
            They can be paid 48 hours after you add them. The wait protects you if someone else gets your key.
          </DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-5" noValidate>
          <div className="space-y-2">
            <div className="flex items-baseline justify-between">
              <Label htmlFor="name">Name</Label>
              <span className={`tabular text-sm ${nameTooLong ? 'text-rust' : 'text-moss'}`} aria-live="polite">
                {nameBytes}/32
              </span>
            </div>
            <Input
              id="name"
              placeholder="Alice (daughter)"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
              autoComplete="off"
              aria-invalid={nameTooLong || undefined}
              aria-describedby="name-hint"
            />
            <p id="name-hint" className={`text-sm ${nameTooLong ? 'text-rust' : 'text-moss'}`}>
              {nameTooLong ? 'Too long. Names are stored on-chain and fit 32 bytes.' : 'Only you see this name.'}
            </p>
          </div>
          <div className="space-y-2">
            <Label htmlFor="address">Their Solana address</Label>
            <Input
              id="address"
              placeholder="e.g. 7xKp…3f2a"
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              required
              autoComplete="off"
              spellCheck={false}
              className="font-mono text-sm"
              aria-invalid={addressInvalid || undefined}
              aria-describedby="address-hint"
            />
            <p id="address-hint" className={`text-sm ${addressInvalid ? 'text-rust' : 'text-moss'}`}>
              {addressInvalid
                ? "That isn't a Solana address. Check for missing characters."
                : 'Ask them to copy it from their wallet.'}
            </p>
          </div>
          <Button type="submit" size="lg" className="w-full" disabled={!canSubmit}>
            {loading ? (
              <>
                <Loader2 className="animate-spin" /> Adding
              </>
            ) : (
              'Add family member'
            )}
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  )
}
