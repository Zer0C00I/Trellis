'use client';

import { useEffect, useState } from 'react';
import { PublicKey } from '@solana/web3.js';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import type {
  TrustedApproversAccount,
  PendingRotationAccount,
} from '@/lib/types';
import { getRotationStatus, truncateAddress } from '@/lib/types';
import { ROTATION_TIMELOCK_HOURS } from '@/lib/constants';
import {
  useRequestRotation,
  useExecuteRotation,
  useCancelRotation,
} from '@/hooks/use-transactions';
import {
  Shield,
  Key,
  Clock,
  AlertTriangle,
  XCircle,
  CheckCircle2,
  RotateCcw,
  Users,
  AlertCircle,
} from 'lucide-react';

interface SecurityRecoveryProps {
  approvers: TrustedApproversAccount | null;
  rotations: PendingRotationAccount[] | null;
  loading: boolean;
  onRefresh: () => void;
}

export function SecurityRecovery({
  approvers,
  rotations,
  loading,
  onRefresh,
}: SecurityRecoveryProps) {
  const [rotationDialogOpen, setRotationDialogOpen] = useState(false);
  const [cancelDialogOpen, setCancelDialogOpen] = useState(false);
  const [selectedRotation, setSelectedRotation] =
    useState<PendingRotationAccount | null>(null);
  const [newOwnerInput, setNewOwnerInput] = useState('');
  const [inputError, setInputError] = useState<string | null>(null);
  const [, setTick] = useState(0);

  const { requestRotation, loading: requesting } = useRequestRotation();
  const { executeRotation, loading: executing } = useExecuteRotation();
  const { cancelRotation, loading: cancelling } = useCancelRotation();

  // Update countdown every minute
  useEffect(() => {
    const interval = setInterval(() => setTick((t) => t + 1), 60_000);
    return () => clearInterval(interval);
  }, []);

  const handleRequestRotation = async () => {
    try {
      const newOwner = new PublicKey(newOwnerInput);
      setInputError(null);
      const result = await requestRotation(newOwner, PublicKey.default);
      if (result) {
        setNewOwnerInput('');
        setRotationDialogOpen(false);
        onRefresh();
      }
    } catch {
      setInputError('Invalid Solana public key');
    }
  };

  const handleExecuteRotation = async (r: PendingRotationAccount) => {
    if (r.publicKey) {
      await executeRotation(r.publicKey, PublicKey.default);
      onRefresh();
    }
  };

  const handleCancelRotation = async () => {
    if (selectedRotation?.publicKey) {
      await cancelRotation(selectedRotation.publicKey, PublicKey.default);
      setCancelDialogOpen(false);
      setSelectedRotation(null);
      onRefresh();
    }
  };

  if (loading) {
    return (
      <div className="space-y-4">
        {[1, 2].map((i) => (
          <div
            key={i}
            className="h-32 rounded-xl bg-white/5 border border-white/10 animate-pulse"
          />
        ))}
      </div>
    );
  }

  const activeRotations = (rotations || []).filter((r) => !r.cancelled);
  const cancelledRotations = (rotations || []).filter((r) => r.cancelled);

  return (
    <div className="space-y-8">
      {/* ─── Trusted Approvers ─────────────────────────────────────────── */}
      <section className="space-y-4">
        <div>
          <h3 className="text-sm font-semibold flex items-center gap-2">
            <Shield className="w-4 h-4 text-purple-400" />
            Trusted Approvers
          </h3>
          <p className="text-xs text-muted-foreground mt-0.5">
            Family members authorized to approve over-limit withdrawals and
            initiate recovery.
          </p>
        </div>

        <div className="grid gap-2">
          {approvers?.entries.map((addr, idx) => (
            <div
              key={addr.toBase58()}
              className="flex items-center gap-3 p-3 rounded-xl border border-white/10 bg-card/50 hover:bg-white/5 transition-colors"
            >
              <div className="w-8 h-8 rounded-lg bg-purple-500/15 flex items-center justify-center text-xs font-bold text-purple-400">
                {idx + 1}
              </div>
              <div className="flex-1">
                <p className="font-mono text-sm">
                  {truncateAddress(addr.toBase58(), 8)}
                </p>
                <p className="text-xs text-muted-foreground">
                  Approver #{idx + 1}
                </p>
              </div>
              <Users className="w-4 h-4 text-muted-foreground/50" />
            </div>
          ))}

          {(!approvers || approvers.entries.length === 0) && (
            <div className="text-center py-8 text-muted-foreground">
              <Shield className="w-8 h-8 mx-auto mb-2 opacity-30" />
              <p className="text-sm">No trusted approvers configured</p>
            </div>
          )}
        </div>
      </section>

      {/* ─── Owner Rotation / Recovery ─────────────────────────────────── */}
      <section className="space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-sm font-semibold flex items-center gap-2">
              <Key className="w-4 h-4 text-amber-400" />
              Owner Recovery
            </h3>
            <p className="text-xs text-muted-foreground mt-0.5">
              Replace the vault owner key if the device is lost or
              compromised.
            </p>
          </div>
          <Button
            size="sm"
            variant="outline"
            onClick={() => setRotationDialogOpen(true)}
            className="border-red-500/30 text-red-400 hover:bg-red-500/10 hover:text-red-300"
          >
            <RotateCcw className="w-4 h-4 mr-1.5" />
            Request Rotation
          </Button>
        </div>

        {/* Active Rotations */}
        {activeRotations.length > 0 && (
          <div className="space-y-3">
            {activeRotations.map((r, idx) => {
              const status = getRotationStatus(r);

              return (
                <div
                  key={r.publicKey?.toBase58() || idx}
                  className={`relative overflow-hidden rounded-xl border p-4 space-y-3 ${
                    status.isReady
                      ? 'border-emerald-500/30 bg-emerald-500/5'
                      : 'border-amber-500/20 bg-amber-500/5'
                  }`}
                >
                  <div
                    className={`absolute top-0 left-0 w-1 h-full ${
                      status.isReady ? 'bg-emerald-500' : 'bg-amber-500'
                    }`}
                  />

                  <div className="pl-3 space-y-3">
                    <div className="flex items-start justify-between">
                      <div className="space-y-1.5">
                        <div className="flex items-center gap-2">
                          {status.isReady ? (
                            <CheckCircle2 className="w-4 h-4 text-emerald-400" />
                          ) : (
                            <Clock className="w-4 h-4 text-amber-400" />
                          )}
                          <span
                            className={`text-sm font-semibold ${
                              status.isReady
                                ? 'text-emerald-400'
                                : 'text-amber-400'
                            }`}
                          >
                            {status.label}
                          </span>
                        </div>
                        <div className="text-xs text-muted-foreground space-y-0.5">
                          <p>
                            New Owner:{' '}
                            <span className="font-mono text-foreground">
                              {truncateAddress(r.newOwner.toBase58(), 6)}
                            </span>
                          </p>
                          <p>
                            Requested by:{' '}
                            <span className="font-mono text-foreground">
                              {truncateAddress(
                                r.requestedBy.toBase58(),
                                6
                              )}
                            </span>
                          </p>
                        </div>
                      </div>

                      <div className="flex items-center gap-2">
                        {status.isReady && (
                          <Button
                            size="sm"
                            onClick={() => handleExecuteRotation(r)}
                            disabled={executing}
                            className="bg-gradient-to-r from-emerald-600 to-emerald-500 text-white border-0"
                          >
                            <CheckCircle2 className="w-4 h-4 mr-1.5" />
                            Execute
                          </Button>
                        )}
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => {
                            setSelectedRotation(r);
                            setCancelDialogOpen(true);
                          }}
                          disabled={cancelling}
                          className="border-red-500/30 text-red-400 hover:bg-red-500/10"
                        >
                          <XCircle className="w-4 h-4 mr-1.5" />
                          Cancel
                        </Button>
                      </div>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}

        {/* Cancelled History */}
        {cancelledRotations.length > 0 && (
          <div className="space-y-2">
            <h4 className="text-xs text-muted-foreground uppercase tracking-wider">
              Cancelled Rotations
            </h4>
            {cancelledRotations.map((r, idx) => (
              <div
                key={r.publicKey?.toBase58() || idx}
                className="rounded-xl border border-white/5 bg-card/20 p-3 flex items-center gap-3 opacity-60"
              >
                <XCircle className="w-4 h-4 text-red-400/50" />
                <span className="text-xs text-muted-foreground">
                  Rotation to {truncateAddress(r.newOwner.toBase58(), 4)} —
                  cancelled
                </span>
              </div>
            ))}
          </div>
        )}

        {activeRotations.length === 0 && cancelledRotations.length === 0 && (
          <div className="text-center py-8 text-muted-foreground border border-white/5 rounded-xl bg-card/20">
            <Key className="w-8 h-8 mx-auto mb-2 opacity-30" />
            <p className="text-sm">No rotation requests</p>
            <p className="text-xs mt-1 opacity-70">
              Use &ldquo;Request Rotation&rdquo; to initiate a key recovery
            </p>
          </div>
        )}
      </section>

      {/* ─── Request Rotation Dialog ──────────────────────────────────── */}
      <Dialog open={rotationDialogOpen} onOpenChange={setRotationDialogOpen}>
        <DialogContent className="bg-card/95 backdrop-blur-xl border-white/10 max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-red-400">
              <AlertTriangle className="w-5 h-5" />
              Request Owner Rotation
            </DialogTitle>
            <DialogDescription>
              This initiates a vault ownership transfer with a{' '}
              {ROTATION_TIMELOCK_HOURS}-hour security timelock.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <Alert className="bg-red-500/10 border-red-500/20 text-red-300">
              <AlertTriangle className="h-4 w-4 text-red-400" />
              <AlertDescription className="text-xs space-y-1">
                <p className="font-semibold">⚠️ Critical Security Action</p>
                <p>
                  This will transfer full vault ownership to a new key after
                  a {ROTATION_TIMELOCK_HOURS}-hour waiting period. During
                  this time, the current owner or any other trusted approver
                  can cancel the rotation.
                </p>
                <p>
                  Only use this for device recovery or key compromise
                  scenarios.
                </p>
              </AlertDescription>
            </Alert>

            <div className="space-y-2">
              <Label htmlFor="new-owner" className="text-sm">
                New Owner Public Key
              </Label>
              <Input
                id="new-owner"
                placeholder="Enter the new owner's Solana public key..."
                value={newOwnerInput}
                onChange={(e) => {
                  setNewOwnerInput(e.target.value);
                  setInputError(null);
                }}
                className="bg-white/5 border-white/10 font-mono text-sm"
              />
              {inputError && (
                <p className="text-xs text-red-400 flex items-center gap-1">
                  <AlertCircle className="w-3 h-3" />
                  {inputError}
                </p>
              )}
            </div>

            <div className="p-3 rounded-lg bg-white/5 text-xs text-muted-foreground space-y-1">
              <p>
                <span className="font-medium text-foreground">
                  Timelock:
                </span>{' '}
                {ROTATION_TIMELOCK_HOURS} hours
              </p>
              <p>
                <span className="font-medium text-foreground">
                  Can cancel:
                </span>{' '}
                Current owner or any trusted approver
              </p>
              <p>
                <span className="font-medium text-foreground">
                  Execute:
                </span>{' '}
                Anyone can execute after timelock expires (permissionless)
              </p>
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setRotationDialogOpen(false)}
              className="border-white/10"
            >
              Cancel
            </Button>
            <Button
              onClick={handleRequestRotation}
              disabled={requesting || !newOwnerInput}
              className="bg-red-600 hover:bg-red-700 text-white border-0"
            >
              {requesting
                ? 'Requesting...'
                : `Request ${ROTATION_TIMELOCK_HOURS}h Rotation`}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* ─── Cancel Confirmation Dialog ───────────────────────────────── */}
      <Dialog open={cancelDialogOpen} onOpenChange={setCancelDialogOpen}>
        <DialogContent className="bg-card/95 backdrop-blur-xl border-white/10 max-w-sm">
          <DialogHeader>
            <DialogTitle>Cancel Owner Rotation?</DialogTitle>
            <DialogDescription>
              This will permanently cancel the pending ownership transfer to{' '}
              {selectedRotation
                ? truncateAddress(
                    selectedRotation.newOwner.toBase58(),
                    6
                  )
                : '—'}
              . This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setCancelDialogOpen(false)}
              className="border-white/10"
            >
              Keep Rotation
            </Button>
            <Button
              onClick={handleCancelRotation}
              disabled={cancelling}
              className="bg-red-600 hover:bg-red-700 text-white border-0"
            >
              {cancelling ? 'Cancelling...' : 'Cancel Rotation'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
