import { AppLayout } from '@/components/app-layout'
import React from 'react'

const links: { label: string; path: string }[] = [{ label: 'Dashboard', path: '/dashboard' }]

export default function DashboardLayout({ children }: { children: React.ReactNode }) {
  return <AppLayout links={links}>{children}</AppLayout>
}
