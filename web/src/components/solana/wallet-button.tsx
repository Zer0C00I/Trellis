'use client';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';
import { useEffect, useState } from 'react';

export function WalletButton() {
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);
  
  if (!mounted) {
    return <div className="h-10 w-[130px] bg-white/5 rounded-md animate-pulse" />;
  }
  
  return <WalletMultiButton />;
}
