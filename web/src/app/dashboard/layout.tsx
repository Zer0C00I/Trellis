import { AppLayout } from '@/components/app-layout'
import React from 'react'

const links: { label: string; path: string }[] = [
  { label: 'Vault', path: '/dashboard' },
  { label: 'Docs', path: '/docs' },
]

export default function DashboardLayout({ children }: { children: React.ReactNode }) {
  return <AppLayout links={links}>{children}</AppLayout>
}
