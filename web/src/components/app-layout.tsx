'use client'

import { AppHeader } from '@/components/app-header'
import React from 'react'
import { AppFooter } from '@/components/app-footer'
import { ClusterChecker } from '@/components/cluster/cluster-ui'

export function AppLayout({
  children,
  links,
}: {
  children: React.ReactNode
  links: { label: string; path: string }[]
}) {
  return (
    <div className="flex min-h-screen flex-col">
      <AppHeader links={links} />
      <main id="main" className="mx-auto w-full max-w-[1200px] flex-grow px-4 sm:px-6">
        <ClusterChecker>{children}</ClusterChecker>
      </main>
      <AppFooter />
    </div>
  )
}
