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
    return <div className="h-10 w-[130px] bg-white/5 rounded-md animate-pulse" />
  }

  return <WalletMultiButton />
}
