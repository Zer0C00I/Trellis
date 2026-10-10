import { useState } from 'react';
import { PublicKey } from '@solana/web3.js';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '../ui/dialog';
import { Button } from '../ui/button';
import { useAnchorProvider } from '../../config/useAnchorProvider';
import { toast } from 'sonner';
import { getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { USDC_MINT } from '../../config/solana';
import { Loader2 } from 'lucide-react';

interface GuardianApprovalModalProps {
  request: any | null; // ProgramAccount of PendingWithdrawal
  isOpen: boolean;
  onClose: () => void;
  onSuccess?: () => void;
}

export function GuardianApprovalModal({ request, isOpen, onClose, onSuccess }: GuardianApprovalModalProps) {
  const { program, provider } = useAnchorProvider();
  const [isApproving, setIsApproving] = useState(false);

  const handleApprove = async () => {
    if (!program || !request || !provider || !provider.wallet) return;

    setIsApproving(true);
    const toastId = toast.loading('Sending approval transaction...');

    try {
      const signer = provider.wallet.publicKey;
      const vault: PublicKey = request.account.vault;

      // The vault records its own token account; read it rather than re-deriving.
      const vaultAccount = await program.account.vault.fetch(vault);
      const vaultTokenAccount = vaultAccount.vaultTokenAccount;

      const recipientTokenAccount = getAssociatedTokenAddressSync(USDC_MINT, request.account.to, true);

      const [approversPda] = PublicKey.findProgramAddressSync(
        [Buffer.from("approvers"), vault.toBuffer()],
        program.programId
      );

      const tx = await program.methods
        .approveWithdrawal()
        .accountsPartial({
          signer: signer,
          pendingWithdrawal: request.publicKey,
          approvers: approversPda,
          vaultTokenAccount: vaultTokenAccount,
          recipientTokenAccount: recipientTokenAccount,
          vault: vault,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
      
      toast.success(`Withdrawal approved successfully! Tx: ${tx.slice(0, 8)}...`, { id: toastId });
      if (onSuccess) onSuccess();
      onClose();
    } catch (error: unknown) {
      console.error(error);
      const message = error instanceof Error ? error.message : 'Unknown error';
      toast.error(`Approval failed: ${message}`, { id: toastId });
    } finally {
      setIsApproving(false);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[425px] bg-zinc-950 border border-zinc-800">
        <DialogHeader>
          <DialogTitle className="text-zinc-100 flex items-center gap-2">
            <span className="h-2 w-2 rounded-full bg-yellow-500 animate-pulse" />
            Guardian Approval Required
          </DialogTitle>
          <DialogDescription className="text-zinc-400 pt-2">
            A withdrawal request has exceeded the daily limit and requires your multi-sig approval to execute.
          </DialogDescription>
        </DialogHeader>
        
        {request && (
          <div className="grid gap-4 py-4">
            <div className="flex flex-col gap-1 p-4 bg-zinc-900/50 rounded-lg border border-zinc-800">
              <span className="text-xs text-zinc-500 font-medium uppercase tracking-wider">Amount Requested</span>
              <span className="text-2xl font-bold text-zinc-100">
                ${(request.account.amount.toNumber() / 1_000_000).toLocaleString()} USDC
              </span>
            </div>
            
            <div className="grid grid-cols-4 items-center gap-4">
              <span className="text-sm font-medium text-zinc-400">Recipient:</span>
              <span className="col-span-3 text-xs text-zinc-200 font-mono">
                {request.account.to.toString()}
              </span>
            </div>
            <div className="grid grid-cols-4 items-center gap-4">
              <span className="text-sm font-medium text-zinc-400">Request ID:</span>
              <span className="col-span-3 text-xs text-zinc-500 font-mono">
                {request.publicKey.toString()}
              </span>
            </div>
          </div>
        )}
        
        <DialogFooter className="gap-2 sm:gap-0">
          <Button variant="ghost" onClick={onClose} disabled={isApproving} className="text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800">
            Cancel
          </Button>
          <Button onClick={handleApprove} disabled={isApproving} className="bg-emerald-600 hover:bg-emerald-700 text-white">
            {isApproving ? (
              <span className="flex items-center gap-2">
                <Loader2 className="h-4 w-4 animate-spin" /> Approving...
              </span>
            ) : 'Sign & Approve'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
