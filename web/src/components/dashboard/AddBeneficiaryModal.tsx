import { useState } from 'react'
import { PublicKey } from '@solana/web3.js'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from '../ui/dialog'
import { Button } from '../ui/button'
import { Input } from '../ui/input'
import { Label } from '../ui/label'
import { useAnchorProvider } from '../../config/useAnchorProvider'
import { toast } from 'sonner'
import { Loader2 } from 'lucide-react'

export function AddBeneficiaryModal({ onAdded }: { onAdded: () => void }) {
  const { program, provider } = useAnchorProvider()
  const [open, setOpen] = useState(false)
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

      // Derive the vault PDA
      const [vaultPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('vault'), owner.toBuffer(), Buffer.from('personal')],
        program.programId,
      )

      // Derive the beneficiary list PDA
      const [beneficiaryListPda] = PublicKey.findProgramAddressSync(
        [Buffer.from('beneficiaries'), vaultPda.toBuffer()],
        program.programId,
      )

      // Submit transaction via Anchor RPC
      const tx = await program.methods
        .addBeneficiary(name.trim(), targetAddress)
        .accountsPartial({
          signer: owner,
          vault: vaultPda,
          beneficiaryList: beneficiaryListPda,
        })
        .rpc()

      toast.success(`Beneficiary added! Tx: ${tx.slice(0, 8)}...`)
      setOpen(false)
      setName('')
      setAddress('')
      onAdded()
    } catch (err: unknown) {
      console.error(err)
      const message = err instanceof Error ? err.message : String(err)
      if (message.includes('AccountNotInitialized')) {
        toast.error('Vault is not initialized! Please create a vault first.')
      } else {
        toast.error(message || 'Failed to add beneficiary')
      }
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button size="sm" className="bg-zinc-100 text-zinc-900 hover:bg-zinc-200">
          + Add Beneficiary
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-[425px] border-zinc-800 bg-zinc-950">
        <DialogHeader>
          <DialogTitle className="text-zinc-100">Add New Beneficiary</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4 mt-4">
          <div className="space-y-2">
            <Label htmlFor="name" className="text-zinc-400">
              Beneficiary Name
            </Label>
            <Input
              id="name"
              placeholder="e.g. Alice (Daughter)"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
              maxLength={32}
              className="bg-zinc-900 border-zinc-800 text-zinc-100 placeholder:text-zinc-600"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="address" className="text-zinc-400">
              Solana Wallet Address
            </Label>
            <Input
              id="address"
              placeholder="Solana Pubkey"
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              required
              className="bg-zinc-900 border-zinc-800 text-zinc-100 placeholder:text-zinc-600"
            />
          </div>
          <Button
            type="submit"
            className="w-full bg-emerald-500 hover:bg-emerald-600 text-white mt-2"
            disabled={loading}
          >
            {loading ? (
              <span className="flex items-center gap-2">
                <Loader2 className="h-4 w-4 animate-spin" /> Confirming...
              </span>
            ) : (
              'Add Beneficiary'
            )}
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  )
}
