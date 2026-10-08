'use client';

import { formatUsdc } from '@/lib/types';
import type { VaultAccount, VaultType } from '@/lib/types';
import { U64_MAX } from '@/lib/constants';
import {
  DollarSign,
  TrendingUp,
  Clock,
  ArrowUpRight,
  Infinity,
} from 'lucide-react';
import { useState } from 'react';
import { DepositModal } from './deposit-modal';

interface FinancialOverviewProps {
  vault: VaultAccount | null;
  balance: bigint | null;
  vaultType: VaultType;
  loading: boolean;
}

export function FinancialOverview({
  vault,
  balance,
  vaultType,
  loading,
}: FinancialOverviewProps) {
  const [depositOpen, setDepositOpen] = useState(false);
  const isPool = vaultType === 'pool';

  if (loading) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {[1, 2, 3].map((i) => (
          <div
            key={i}
            className="h-32 rounded-xl bg-white/5 border border-white/10 animate-pulse"
          />
        ))}
      </div>
    );
  }

  const balanceFormatted = balance !== null ? formatUsdc(balance) : '—';
  const dailyLimitFormatted =
    vault && vault.dailyLimit !== U64_MAX
      ? formatUsdc(vault.dailyLimit)
      : null;
  const spentTodayFormatted = vault ? formatUsdc(vault.spentToday) : '0.00';
  const dailyProgress =
    vault && vault.dailyLimit !== U64_MAX && vault.dailyLimit > BigInt(0)
      ? Number(vault.spentToday * BigInt(100) / vault.dailyLimit)
      : 0;

  return (
    <>
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {/* Balance Card */}
        <div className="relative group overflow-hidden rounded-xl border border-white/10 bg-gradient-to-br from-purple-500/10 to-emerald-500/10 p-5">
          <div className="absolute inset-0 bg-gradient-to-br from-purple-500/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500" />
          <div className="relative space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                USDC Balance
              </span>
              <div className="w-8 h-8 rounded-lg bg-purple-500/20 flex items-center justify-center">
                <DollarSign className="w-4 h-4 text-purple-400" />
              </div>
            </div>
            <div className="flex items-baseline gap-1">
              <span className="text-3xl font-bold tracking-tight">
                ${balanceFormatted}
              </span>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => setDepositOpen(true)}
                className="flex items-center gap-1 text-xs text-emerald-400 hover:text-emerald-300 transition-colors font-medium"
              >
                <ArrowUpRight className="w-3 h-3" />
                Deposit USDC
              </button>
            </div>
          </div>
        </div>

        {/* Daily Limit Card (Pool only) */}
        <div className="relative overflow-hidden rounded-xl border border-white/10 bg-card/50 p-5">
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                Daily Limit
              </span>
              <div className="w-8 h-8 rounded-lg bg-emerald-500/20 flex items-center justify-center">
                {isPool ? (
                  <TrendingUp className="w-4 h-4 text-emerald-400" />
                ) : (
                  <Infinity className="w-4 h-4 text-emerald-400" />
                )}
              </div>
            </div>

            {isPool && dailyLimitFormatted ? (
              <>
                <div className="flex items-baseline gap-1">
                  <span className="text-3xl font-bold tracking-tight">
                    ${dailyLimitFormatted}
                  </span>
                </div>
                {/* Progress bar */}
                <div className="space-y-1.5">
                  <div className="h-2 rounded-full bg-white/10 overflow-hidden">
                    <div
                      className={`h-full rounded-full transition-all duration-700 ${
                        dailyProgress > 80
                          ? 'bg-gradient-to-r from-red-500 to-red-400'
                          : dailyProgress > 50
                          ? 'bg-gradient-to-r from-amber-500 to-amber-400'
                          : 'bg-gradient-to-r from-emerald-500 to-emerald-400'
                      }`}
                      style={{ width: `${Math.min(dailyProgress, 100)}%` }}
                    />
                  </div>
                  <p className="text-xs text-muted-foreground">
                    ${spentTodayFormatted} spent today ({dailyProgress}%)
                  </p>
                </div>
              </>
            ) : (
              <>
                <p className="text-2xl font-bold text-emerald-400">
                  No Limit
                </p>
                <p className="text-xs text-muted-foreground">
                  Personal vault — her money, her control
                </p>
              </>
            )}
          </div>
        </div>

        {/* Status Card */}
        <div className="relative overflow-hidden rounded-xl border border-white/10 bg-card/50 p-5">
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                Vault Status
              </span>
              <div className="w-8 h-8 rounded-lg bg-blue-500/20 flex items-center justify-center">
                <Clock className="w-4 h-4 text-blue-400" />
              </div>
            </div>
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
                <span className="text-sm font-medium">Active</span>
              </div>
              <div className="text-xs text-muted-foreground space-y-1">
                <p>
                  Type:{' '}
                  <span className="font-medium text-foreground">
                    {isPool ? 'Family Pool' : 'Personal'}
                  </span>
                </p>
                <p>
                  Network:{' '}
                  <span className="font-medium text-foreground">
                    Solana Devnet
                  </span>
                </p>
                <p>
                  Token:{' '}
                  <span className="font-medium text-foreground">USDC</span>
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>

      <DepositModal
        open={depositOpen}
        onOpenChange={setDepositOpen}
        vaultType={vaultType}
      />
    </>
  );
}
