import { useState, useEffect } from 'react';
import { PublicKey } from '@solana/web3.js';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../ui/table';
import { Badge } from '../ui/badge';
import { useAnchorProvider } from '../../config/useAnchorProvider';

interface Beneficiary {
  publicKey: PublicKey;
  name: string;
  address: PublicKey;
  activeAfter: number; // Unix timestamp; payee can be paid from this moment
}

interface BeneficiaryTableProps {
  vaultPda: PublicKey;
  refreshCounter: number;
}

export function BeneficiaryTable({ vaultPda, refreshCounter }: BeneficiaryTableProps) {
  const { program } = useAnchorProvider();
  const [beneficiaries, setBeneficiaries] = useState<Beneficiary[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!program) return;
    let cancelled = false;

    const fetchBeneficiaries = async () => {
      try {
        setLoading(true);
        // Only this vault's list (PDA: ["beneficiaries", vault]).
        const [listPda] = PublicKey.findProgramAddressSync(
          [Buffer.from('beneficiaries'), vaultPda.toBuffer()],
          program.programId
        );
        const list = await program.account.beneficiaryList.fetchNullable(listPda);

        const result: Beneficiary[] = [];
        if (list) {
          for (let i = 0; i < list.count; i++) {
            const entry = list.entries[i];
            const nameBytes = Uint8Array.from(entry.name);
            const nameStr = new TextDecoder().decode(nameBytes.slice(0, entry.nameLen));
            result.push({
              publicKey: entry.address,
              name: nameStr,
              address: entry.address,
              activeAfter: entry.activeAfter.toNumber(),
            });
          }
        }
        if (!cancelled) setBeneficiaries(result);
      } catch (err) {
        console.error('Failed to fetch beneficiaries:', err);
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    fetchBeneficiaries();
    return () => {
      cancelled = true;
    };
  }, [program, vaultPda, refreshCounter]);

  // The program enforces the cooldown itself and stores the exact moment the
  // payee becomes usable, so show that rather than recomputing 48h client-side.
  const getStatusBadge = (activeAfter: number) => {
    const remainingSecs = activeAfter - Date.now() / 1000;
    if (remainingSecs <= 0) {
      return <Badge className="bg-emerald-500/20 text-emerald-500 border-emerald-500/50">🟢 Active</Badge>;
    }
    const remainingHours = Math.ceil(remainingSecs / 3600);
    return <Badge variant="outline" className="text-yellow-500 border-yellow-500/50">🟡 Cooldown Pending: {remainingHours}h remaining</Badge>;
  };

  return (
    <div className="rounded-md border border-zinc-800 bg-zinc-950/50">
      <Table>
        <TableHeader>
          <TableRow className="border-zinc-800 hover:bg-transparent">
            <TableHead className="text-zinc-400">Name</TableHead>
            <TableHead className="text-zinc-400">Address</TableHead>
            <TableHead className="text-zinc-400">Status</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {loading ? (
            <TableRow>
              <TableCell colSpan={3} className="text-center text-zinc-500 py-8">Loading beneficiaries...</TableCell>
            </TableRow>
          ) : beneficiaries.length === 0 ? (
            <TableRow>
              <TableCell colSpan={3} className="text-center text-zinc-500 py-8">No beneficiaries registered yet.</TableCell>
            </TableRow>
          ) : (
            beneficiaries.map((b) => (
              <TableRow key={b.publicKey.toString()} className="border-zinc-800">
                <TableCell className="font-medium text-zinc-200">{b.name}</TableCell>
                <TableCell className="font-mono text-sm text-zinc-400">
                  {b.address.toString().slice(0, 4)}...{b.address.toString().slice(-4)}
                </TableCell>
                <TableCell>{getStatusBadge(b.activeAfter)}</TableCell>
              </TableRow>
            ))
          )}
        </TableBody>
      </Table>
    </div>
  );
}
