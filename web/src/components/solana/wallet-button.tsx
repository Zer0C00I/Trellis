'use client'
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui'
import { useSyncExternalStore } from 'react'

export function WalletButton() {
  // false during server render and hydration, true once mounted in the browser
  const mounted = useSyncExternalStore(
    () => () => {},
    () => true,
    () => false,
  )

  if (!mounted) {
    return <div className="h-10 w-[140px] animate-pulse rounded-md bg-ivy/10" />
  }

  return <WalletMultiButton />
}
