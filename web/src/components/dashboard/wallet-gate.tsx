'use client';

import { useWallet } from '@solana/wallet-adapter-react';
import { WalletButton } from '@/components/solana/wallet-button';
import { Shield, Lock, Users } from 'lucide-react';

export function WalletGate({ children }: { children: React.ReactNode }) {
  const { connected } = useWallet();

  if (connected) return <>{children}</>;

  return (
    <div className="min-h-[80vh] flex items-center justify-center">
      <div className="relative max-w-lg w-full mx-4">
        {/* Glow background */}
        <div className="absolute -inset-1 bg-gradient-to-r from-purple-600 via-emerald-500 to-purple-600 rounded-2xl blur-xl opacity-20 animate-pulse" />

        <div className="relative bg-card/80 backdrop-blur-xl border border-white/10 rounded-2xl p-10 text-center space-y-8">
          {/* Logo */}
          <div className="flex justify-center">
            <div className="relative">
              <div className="w-20 h-20 rounded-2xl bg-gradient-to-br from-purple-500 to-emerald-400 flex items-center justify-center shadow-lg shadow-purple-500/25">
                <Shield className="w-10 h-10 text-white" />
              </div>
              <div className="absolute -bottom-1 -right-1 w-7 h-7 rounded-lg bg-emerald-500 flex items-center justify-center border-2 border-card">
                <Lock className="w-3.5 h-3.5 text-white" />
              </div>
            </div>
          </div>

          <div className="space-y-3">
            <h1 className="text-3xl font-bold bg-gradient-to-r from-purple-400 to-emerald-400 bg-clip-text text-transparent">
              Trellis
            </h1>
            <p className="text-muted-foreground text-sm leading-relaxed max-w-sm mx-auto">
              Relative&apos;s Management Portal — secure administration for
              your family&apos;s financial safety on Solana.
            </p>
          </div>

          {/* Features */}
          <div className="grid grid-cols-3 gap-3 text-xs text-muted-foreground">
            <div className="flex flex-col items-center gap-1.5 p-3 rounded-lg bg-white/5">
              <Shield className="w-4 h-4 text-purple-400" />
              <span>Anti-Scam</span>
            </div>
            <div className="flex flex-col items-center gap-1.5 p-3 rounded-lg bg-white/5">
              <Users className="w-4 h-4 text-emerald-400" />
              <span>Family Pool</span>
            </div>
            <div className="flex flex-col items-center gap-1.5 p-3 rounded-lg bg-white/5">
              <Lock className="w-4 h-4 text-purple-400" />
              <span>Closed Book</span>
            </div>
          </div>

          <div className="pt-2">
            <WalletButton />
          </div>

          <p className="text-xs text-muted-foreground/60">
            Connect your wallet to access the admin dashboard
          </p>
        </div>
      </div>
    </div>
  );
}

