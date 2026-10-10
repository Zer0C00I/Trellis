import type { Metadata } from 'next'
import { JetBrains_Mono, Onest } from 'next/font/google'
import './globals.css'
import { AppProviders } from '@/components/app-providers'
import { Toaster } from '@/components/ui/sonner'
import React from 'react'

const onest = Onest({
  subsets: ['latin', 'cyrillic'],
  variable: '--font-onest',
  display: 'swap',
})

// Only for wallet addresses and signatures, where telling l from 1 matters.
const jetbrains = JetBrains_Mono({
  subsets: ['latin'],
  variable: '--font-jetbrains',
  display: 'swap',
})

export const metadata: Metadata = {
  title: 'Trellis: crypto inheritance without middlemen',
  description:
    'A self-custodial USDC vault on Solana. Name who it goes to, cap what leaves each day, and add guardians who sign off on more.',
}

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en" className={`${onest.variable} ${jetbrains.variable}`} suppressHydrationWarning>
      <body className="antialiased">
        <AppProviders>
          {children}
          <Toaster />
        </AppProviders>
      </body>
    </html>
  )
}
// Patch BigInt so we can log it using JSON.stringify without any errors
declare global {
  interface BigInt {
    toJSON(): string
  }
}

BigInt.prototype.toJSON = function () {
  return this.toString()
}
