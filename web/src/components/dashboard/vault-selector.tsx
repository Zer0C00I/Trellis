'use client';

import { useState } from 'react';
import { PublicKey } from '@solana/web3.js';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { MOCK_MODE } from '@/lib/constants';
import type { VaultType } from '@/lib/types';
import { Wallet, Users, AlertCircle } from 'lucide-react';

interface VaultSelectorProps {
  vaultType: VaultType;
  onVaultTypeChange: (type: VaultType) => void;
  elderPubkey: PublicKey | null;
  onElderPubkeyChange: (key: PublicKey | null) => void;
}

export function VaultSelector({
  vaultType,
  onVaultTypeChange,
  elderPubkey,
  onElderPubkeyChange,
}: VaultSelectorProps) {
  const [keyInput, setKeyInput] = useState(elderPubkey?.toBase58() || '');
  const [keyError, setKeyError] = useState<string | null>(null);

  const handleKeySubmit = () => {
    if (MOCK_MODE) {
      onElderPubkeyChange(PublicKey.default);
      return;
    }
    try {
      const key = new PublicKey(keyInput);
      setKeyError(null);
      onElderPubkeyChange(key);
    } catch {
      setKeyError('Invalid Solana public key');
    }
  };

  return (
    <div className="space-y-4">
      {/* Vault Type Toggle */}
      <div className="flex items-center gap-2">
        <Button
          variant={vaultType === 'personal' ? 'default' : 'outline'}
          size="sm"
          onClick={() => onVaultTypeChange('personal')}
          className={
            vaultType === 'personal'
              ? 'bg-gradient-to-r from-purple-600 to-purple-500 text-white shadow-lg shadow-purple-500/25 border-0'
              : 'border-white/10 hover:bg-white/5'
          }
        >
          <Wallet className="w-4 h-4 mr-1.5" />
          Personal Vault
        </Button>
        <Button
          variant={vaultType === 'pool' ? 'default' : 'outline'}
          size="sm"
          onClick={() => onVaultTypeChange('pool')}
          className={
            vaultType === 'pool'
              ? 'bg-gradient-to-r from-emerald-600 to-emerald-500 text-white shadow-lg shadow-emerald-500/25 border-0'
              : 'border-white/10 hover:bg-white/5'
          }
        >
          <Users className="w-4 h-4 mr-1.5" />
          Family Pool
        </Button>
      </div>

      {/* Elder Public Key Input */}
      {!MOCK_MODE && !elderPubkey && (
        <div className="flex items-end gap-3 p-4 rounded-xl bg-white/5 border border-white/10">
          <div className="flex-1 space-y-2">
            <Label className="text-xs text-muted-foreground">
              Elder&apos;s Wallet Address
            </Label>
            <Input
              placeholder="Enter the elder's Solana public key..."
              value={keyInput}
              onChange={(e) => {
                setKeyInput(e.target.value);
                setKeyError(null);
              }}
              className="bg-white/5 border-white/10 text-sm font-mono"
            />
            {keyError && (
              <p className="text-xs text-red-400 flex items-center gap-1">
                <AlertCircle className="w-3 h-3" />
                {keyError}
              </p>
            )}
          </div>
          <Button size="sm" onClick={handleKeySubmit}>
            Load Vault
          </Button>
        </div>
      )}

      {/* Mock mode indicator */}
      {MOCK_MODE && (
        <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-amber-500/10 border border-amber-500/20 text-amber-400 text-xs">
          <AlertCircle className="w-3.5 h-3.5" />
          <span>Demo Mode — using mock data</span>
        </div>
      )}
    </div>
  );
}
