'use client'

import { useState } from 'react'
import { PublicKey } from '@solana/web3.js'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { useAddBeneficiary } from '@/hooks/use-transactions'
import { BENEFICIARY_COOLDOWN_HOURS } from '@/lib/constants'
import { AlertCircle, Clock, UserPlus } from 'lucide-react'

interface AddBeneficiaryFormProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onSuccess: () => void
}

export function AddBeneficiaryForm({ open, onOpenChange, onSuccess }: AddBeneficiaryFormProps) {
  const [name, setName] = useState('')
  const [address, setAddress] = useState('')
  const [nameError, setNameError] = useState<string | null>(null)
  const [addressError, setAddressError] = useState<string | null>(null)
  const { addBeneficiary, loading } = useAddBeneficiary()

  const validate = (): boolean => {
    let valid = true

    if (!name.trim() || name.trim().length < 2) {
      setNameError('Name must be at least 2 characters')
      valid = false
    } else if (name.trim().length > 32) {
      setNameError('Name must be 32 characters or less')
      valid = false
    } else {
      setNameError(null)
    }

    try {
      new PublicKey(address)
      setAddressError(null)
    } catch {
      setAddressError('Invalid Solana public key')
      valid = false
    }

    return valid
  }

  const handleSubmit = async () => {
    if (!validate()) return

    const pubkey = new PublicKey(address)
    const result = await addBeneficiary(
      name.trim(),
      pubkey,
      PublicKey.default, // beneficiaryList PDA placeholder
      PublicKey.default, // vault PDA placeholder
    )

    if (result) {
      setName('')
      setAddress('')
      onSuccess()
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="bg-card/95 backdrop-blur-xl border-white/10 max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-purple-500/20 flex items-center justify-center">
              <UserPlus className="w-4 h-4 text-purple-400" />
            </div>
            Add Beneficiary
          </DialogTitle>
          <DialogDescription>
            Add a new recipient to the closed address book. The beneficiary will be subject to a mandatory cooldown
            period before becoming active.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* 48h Cooldown Warning */}
          <Alert className="bg-amber-500/10 border-amber-500/20 text-amber-300">
            <Clock className="h-4 w-4 text-amber-400" />
            <AlertDescription className="text-xs">
              <span className="font-semibold">{BENEFICIARY_COOLDOWN_HOURS}-hour anti-scam cooldown.</span> This
              beneficiary will not be available for transfers until {BENEFICIARY_COOLDOWN_HOURS} hours after being
              added. This protects against social engineering attacks.
            </AlertDescription>
          </Alert>

          <div className="space-y-2">
            <Label htmlFor="ben-name" className="text-sm">
              Display Name
            </Label>
            <Input
              id="ben-name"
              placeholder="e.g. Pharmacy — CVS"
              value={name}
              onChange={(e) => {
                setName(e.target.value)
                setNameError(null)
              }}
              className="bg-white/5 border-white/10"
            />
            {nameError && (
              <p className="text-xs text-red-400 flex items-center gap-1">
                <AlertCircle className="w-3 h-3" />
                {nameError}
              </p>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="ben-address" className="text-sm">
              Solana Address
            </Label>
            <Input
              id="ben-address"
              placeholder="Enter recipient's Solana public key..."
              value={address}
              onChange={(e) => {
                setAddress(e.target.value)
                setAddressError(null)
              }}
              className="bg-white/5 border-white/10 font-mono text-sm"
            />
            {addressError && (
              <p className="text-xs text-red-400 flex items-center gap-1">
                <AlertCircle className="w-3 h-3" />
                {addressError}
              </p>
            )}
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} className="border-white/10">
            Cancel
          </Button>
          <Button
            onClick={handleSubmit}
            disabled={loading || !name || !address}
            className="bg-gradient-to-r from-purple-600 to-purple-500 text-white border-0"
          >
            {loading ? 'Adding...' : 'Add Beneficiary'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
