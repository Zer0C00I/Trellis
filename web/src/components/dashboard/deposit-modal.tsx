'use client';

import { useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { useDeposit } from '@/hooks/use-transactions';
import { PublicKey } from '@solana/web3.js';
import { ArrowUpRight, AlertCircle } from 'lucide-react';
import type { VaultType } from '@/lib/types';

interface DepositModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  vaultType: VaultType;
}

export function DepositModal({ open, onOpenChange, vaultType }: DepositModalProps) {
  const [amount, setAmount] = useState('');
  const [amountError, setAmountError] = useState<string | null>(null);
  const { deposit, loading } = useDeposit();

  const handleDeposit = async () => {
    const num = parseFloat(amount);
    if (isNaN(num) || num <= 0) {
      setAmountError('Enter a valid amount greater than 0');
      return;
    }
    if (num > 1_000_000) {
      setAmountError('Amount exceeds maximum');
      return;
    }

    setAmountError(null);
    const lamports = BigInt(Math.round(num * 1_000_000));
    const result = await deposit(lamports, PublicKey.default);
    if (result) {
      setAmount('');
      onOpenChange(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="bg-card/95 backdrop-blur-xl border-white/10 max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-emerald-500/20 flex items-center justify-center">
              <ArrowUpRight className="w-4 h-4 text-emerald-400" />
            </div>
            Deposit USDC
          </DialogTitle>
          <DialogDescription>
            {vaultType === 'pool'
              ? 'Fund the family pool. Anyone can deposit — these become shared funds subject to daily limits and approval.'
              : 'Deposit USDC into the personal vault. Only the vault owner can deposit.'}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="deposit-amount" className="text-sm">
              Amount (USDC)
            </Label>
            <div className="relative">
              <span className="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground text-sm">
                $
              </span>
              <Input
                id="deposit-amount"
                type="number"
                step="0.01"
                min="0"
                placeholder="0.00"
                value={amount}
                onChange={(e) => {
                  setAmount(e.target.value);
                  setAmountError(null);
                }}
                className="pl-7 bg-white/5 border-white/10 font-mono"
              />
            </div>
            {amountError && (
              <p className="text-xs text-red-400 flex items-center gap-1">
                <AlertCircle className="w-3 h-3" />
                {amountError}
              </p>
            )}
          </div>

          {/* Quick amounts */}
          <div className="flex gap-2">
            {[50, 100, 250, 500].map((val) => (
              <button
                key={val}
                onClick={() => setAmount(val.toString())}
                className="flex-1 py-1.5 rounded-lg bg-white/5 border border-white/10 text-xs text-muted-foreground hover:bg-white/10 hover:text-foreground transition-colors"
              >
                ${val}
              </button>
            ))}
          </div>

          <div className="p-3 rounded-lg bg-blue-500/10 border border-blue-500/20 text-xs text-blue-300 space-y-1">
            <p className="font-medium">ℹ️ Devnet Transaction</p>
            <p className="text-blue-300/70">
              This deposits devnet USDC. No real funds are transferred.
              Get devnet USDC from the{' '}
              <a
                href="https://faucet.circle.com/"
                target="_blank"
                rel="noopener noreferrer"
                className="underline hover:text-blue-200"
              >
                Circle Faucet
              </a>
              .
            </p>
          </div>
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            className="border-white/10"
          >
            Cancel
          </Button>
          <Button
            onClick={handleDeposit}
            disabled={loading || !amount}
            className="bg-gradient-to-r from-emerald-600 to-emerald-500 text-white border-0"
          >
            {loading ? 'Depositing...' : `Deposit $${amount || '0.00'} USDC`}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
