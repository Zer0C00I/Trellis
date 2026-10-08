'use client';

import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
import type {
  PendingWithdrawalAccount,
  BeneficiaryListAccount,
} from '@/lib/types';
import {
  getWithdrawalStatus,
  formatUsdc,
  truncateAddress,
} from '@/lib/types';
import { useApproveWithdrawal } from '@/hooks/use-transactions';
import {
  AlertTriangle,
  CheckCircle2,
  Clock,
  DollarSign,
  ArrowRight,
} from 'lucide-react';

interface PendingWithdrawalsProps {
  withdrawals: PendingWithdrawalAccount[] | null;
  beneficiaryList: BeneficiaryListAccount | null;
  loading: boolean;
  onRefresh: () => void;
}

export function PendingWithdrawals({
  withdrawals,
  beneficiaryList,
  loading,
  onRefresh,
}: PendingWithdrawalsProps) {
  const { approveWithdrawal, loading: approving } = useApproveWithdrawal();
  const [, setTick] = useState(0);

  useEffect(() => {
    const interval = setInterval(() => setTick((t) => t + 1), 60_000);
    return () => clearInterval(interval);
  }, []);

  const findBeneficiaryName = (address: string): string => {
    const entry = beneficiaryList?.entries.find(
      (b) => b.address.toBase58() === address
    );
    return entry?.name || truncateAddress(address, 4);
  };

  if (loading) {
    return (
      <div className="space-y-4">
        {[1, 2].map((i) => (
          <div
            key={i}
            className="h-28 rounded-xl bg-white/5 border border-white/10 animate-pulse"
          />
        ))}
      </div>
    );
  }

  const items = withdrawals || [];
  const pending = items.filter((w) => !w.executed && !w.approvedBy);
  const completed = items.filter((w) => w.executed || w.approvedBy);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h3 className="text-sm font-semibold">Approval Queue</h3>
        <p className="text-xs text-muted-foreground mt-0.5">
          Withdrawal requests exceeding the daily limit require approval from
          a trusted family member.
        </p>
      </div>

      {/* Pending Requests */}
      {pending.length > 0 && (
        <div className="space-y-3">
          <h4 className="text-xs font-medium text-amber-400 uppercase tracking-wider flex items-center gap-1.5">
            <AlertTriangle className="w-3.5 h-3.5" />
            Requires Attention ({pending.length})
          </h4>
          {pending.map((w, idx) => {
            const recipientName = findBeneficiaryName(w.to.toBase58());
            const requestedAgo = Math.floor(
              (Date.now() - Number(w.requestedAt) * 1000) / 3_600_000
            );

            return (
              <div
                key={w.publicKey?.toBase58() || idx}
                className="relative overflow-hidden rounded-xl border border-amber-500/20 bg-amber-500/5 p-4 space-y-3"
              >
                <div className="absolute top-0 left-0 w-1 h-full bg-amber-500" />

                <div className="flex items-start justify-between pl-3">
                  <div className="space-y-1">
                    <div className="flex items-center gap-2">
                      <DollarSign className="w-4 h-4 text-amber-400" />
                      <span className="text-lg font-bold">
                        ${formatUsdc(w.amount)}
                      </span>
                      <span className="text-xs text-muted-foreground">
                        USDC
                      </span>
                    </div>
                    <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                      <ArrowRight className="w-3 h-3" />
                      <span>To: </span>
                      <span className="font-medium text-foreground">
                        {recipientName}
                      </span>
                    </div>
                    <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                      <Clock className="w-3 h-3" />
                      <span>Requested {requestedAgo}h ago</span>
                    </div>
                  </div>

                  <Button
                    size="sm"
                    onClick={async () => {
                      if (w.publicKey) {
                        await approveWithdrawal(w.publicKey);
                        onRefresh();
                      }
                    }}
                    disabled={approving}
                    className="bg-gradient-to-r from-emerald-600 to-emerald-500 text-white border-0 shadow-lg shadow-emerald-500/20"
                  >
                    <CheckCircle2 className="w-4 h-4 mr-1.5" />
                    Approve
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Completed */}
      {completed.length > 0 && (
        <div className="space-y-3">
          <h4 className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
            History
          </h4>
          {completed.map((w, idx) => {
            const status = getWithdrawalStatus(w);
            const recipientName = findBeneficiaryName(w.to.toBase58());

            return (
              <div
                key={w.publicKey?.toBase58() || idx}
                className="rounded-xl border border-white/10 bg-card/30 p-4 flex items-center justify-between"
              >
                <div className="flex items-center gap-3">
                  <div className="w-8 h-8 rounded-lg bg-emerald-500/15 flex items-center justify-center">
                    <CheckCircle2 className="w-4 h-4 text-emerald-400" />
                  </div>
                  <div>
                    <p className="text-sm font-medium">
                      ${formatUsdc(w.amount)} → {recipientName}
                    </p>
                    <p className="text-xs text-muted-foreground">
                      Approved by{' '}
                      {w.approvedBy
                        ? truncateAddress(w.approvedBy.toBase58(), 4)
                        : '—'}
                    </p>
                  </div>
                </div>
                <span className="inline-flex items-center px-2 py-0.5 rounded-full bg-emerald-500/15 text-emerald-400 text-xs font-medium">
                  {status.label}
                </span>
              </div>
            );
          })}
        </div>
      )}

      {items.length === 0 && (
        <div className="text-center py-12 text-muted-foreground">
          <DollarSign className="w-10 h-10 mx-auto mb-3 opacity-30" />
          <p className="text-sm">No pending withdrawal requests</p>
          <p className="text-xs mt-1 opacity-70">
            Requests appear here when a pool withdrawal exceeds the daily
            limit
          </p>
        </div>
      )}
    </div>
  );
}
