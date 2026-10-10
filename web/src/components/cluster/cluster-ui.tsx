'use client'

import { useConnection } from '@solana/wallet-adapter-react'

import { useQuery } from '@tanstack/react-query'
import * as React from 'react'
import { ReactNode } from 'react'

import { useCluster } from './cluster-data-access'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Button } from '@/components/ui/button'
import { AppAlert } from '@/components/app-alert'

export function ClusterChecker({ children }: { children: ReactNode }) {
  const { cluster } = useCluster()
  const { connection } = useConnection()

  const query = useQuery({
    queryKey: ['version', { cluster, endpoint: connection.rpcEndpoint }],
    queryFn: () => connection.getVersion(),
    retry: 1,
  })
  if (query.isLoading) {
    return null
  }
  if (query.isError || !query.data) {
    return (
      <AppAlert
        action={
          <Button variant="outline" onClick={() => query.refetch()}>
            Try again
          </Button>
        }
      >
        Can&apos;t reach Solana <span className="font-semibold capitalize">{cluster.name}</span>. Check your connection
        or VPN, then try again.
      </AppAlert>
    )
  }
  return children
}

export function ClusterUiSelect() {
  const { clusters, setCluster, cluster } = useCluster()
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" aria-label={`Network: ${cluster.name}. Change network`}>
          <span aria-hidden className="size-2 rounded-full bg-sap" />
          <span className="capitalize">{cluster.name}</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {clusters.map((item) => (
          <DropdownMenuItem key={item.name} onClick={() => setCluster(item)}>
            {item.name}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
