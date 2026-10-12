'use client'

import { useEffect, useState } from 'react'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Button } from '@/components/ui/button'
import type { BeneficiaryListAccount } from '@/lib/types'
import { getBeneficiaryStatus, truncateAddress } from '@/lib/types'
import { MAX_BENEFICIARIES } from '@/lib/constants'
import { Copy, Check, UserPlus, Clock, ShieldCheck } from 'lucide-react'
import { toast } from 'sonner'
import { AddBeneficiaryForm } from './add-beneficiary-form'

interface BeneficiaryTableProps {
  beneficiaryList: BeneficiaryListAccount | null
  loading: boolean
  onRefresh: () => void
}

export function BeneficiaryTable({ beneficiaryList, loading, onRefresh }: BeneficiaryTableProps) {
  const [addOpen, setAddOpen] = useState(false)
  const [copiedIdx, setCopiedIdx] = useState<number | null>(null)
  const [, setTick] = useState(0)

  // Re-render every minute for countdown timers
  useEffect(() => {
    const interval = setInterval(() => setTick((t) => t + 1), 60_000)
    return () => clearInterval(interval)
  }, [])

  const entries = beneficiaryList?.entries || []
  const isFull = entries.length >= MAX_BENEFICIARIES

  const copyAddress = (address: string, idx: number) => {
    navigator.clipboard.writeText(address)
    setCopiedIdx(idx)
    toast.success('Address copied')
    setTimeout(() => setCopiedIdx(null), 2000)
  }

  if (loading) {
    return (
      <div className="rounded-xl border border-white/10 bg-card/50 overflow-hidden">
        <div className="p-4 space-y-3">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-12 rounded-lg bg-white/5 animate-pulse" />
          ))}
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-semibold">Approved Beneficiaries</h3>
          <p className="text-xs text-muted-foreground mt-0.5">
            {entries.length}/{MAX_BENEFICIARIES} slots used — closed address book
          </p>
        </div>
        <Button
          size="sm"
          onClick={() => setAddOpen(true)}
          disabled={isFull}
          className="bg-gradient-to-r from-purple-600 to-purple-500 text-white border-0 shadow-lg shadow-purple-500/20"
        >
          <UserPlus className="w-4 h-4 mr-1.5" />
          Add Beneficiary
        </Button>
      </div>

      <div className="rounded-xl border border-white/10 bg-card/50 overflow-hidden">
        <Table>
          <TableHeader>
            <TableRow className="border-white/10 hover:bg-transparent">
              <TableHead className="text-xs font-medium text-muted-foreground w-10">#</TableHead>
              <TableHead className="text-xs font-medium text-muted-foreground">Name</TableHead>
              <TableHead className="text-xs font-medium text-muted-foreground">Address</TableHead>
              <TableHead className="text-xs font-medium text-muted-foreground">Status</TableHead>
              <TableHead className="text-xs font-medium text-muted-foreground text-right">Added</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {entries.length === 0 ? (
              <TableRow>
                <TableCell colSpan={5} className="h-24 text-center text-muted-foreground text-sm">
                  No beneficiaries added yet
                </TableCell>
              </TableRow>
            ) : (
              entries.map((b, idx) => {
                const status = getBeneficiaryStatus(b)
                const addr = b.address.toBase58()
                const addedDate = new Date(Number(b.addedAt) * 1000).toLocaleDateString('en-US', {
                  month: 'short',
                  day: 'numeric',
                })

                return (
                  <TableRow key={addr} className="border-white/5 hover:bg-white/5 transition-colors">
                    <TableCell className="text-xs text-muted-foreground font-mono">{idx + 1}</TableCell>
                    <TableCell className="font-medium text-sm">{b.name}</TableCell>
                    <TableCell>
                      <button
                        onClick={() => copyAddress(addr, idx)}
                        className="flex items-center gap-1.5 font-mono text-xs text-muted-foreground hover:text-foreground transition-colors group"
                      >
                        {truncateAddress(addr, 6)}
                        {copiedIdx === idx ? (
                          <Check className="w-3 h-3 text-emerald-400" />
                        ) : (
                          <Copy className="w-3 h-3 opacity-0 group-hover:opacity-100 transition-opacity" />
                        )}
                      </button>
                    </TableCell>
                    <TableCell>
                      {status.isActive ? (
                        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-emerald-500/15 text-emerald-400 text-xs font-medium">
                          <ShieldCheck className="w-3 h-3" />
                          Active
                        </span>
                      ) : (
                        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-amber-500/15 text-amber-400 text-xs font-medium">
                          <Clock className="w-3 h-3" />
                          {status.label}
                        </span>
                      )}
                    </TableCell>
                    <TableCell className="text-right text-xs text-muted-foreground">{addedDate}</TableCell>
                  </TableRow>
                )
              })
            )}
          </TableBody>
        </Table>
      </div>

      <AddBeneficiaryForm
        open={addOpen}
        onOpenChange={setAddOpen}
        onSuccess={() => {
          onRefresh()
          setAddOpen(false)
        }}
      />
    </div>
  )
}
