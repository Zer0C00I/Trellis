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
    <div className="flex flex-col min-h-screen bg-black bg-[url('/bg-trellis.jpg')] bg-cover bg-center bg-fixed font-sans selection:bg-blue-500/30 selection:text-blue-200">
      <AppHeader links={links} />
      <main className="flex-grow container mx-auto p-4 md:p-8">
        <ClusterChecker>
          {children}
        </ClusterChecker>
      </main>
      <AppFooter />
    </div>
  )
}
