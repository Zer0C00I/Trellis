'use client'

import Image from 'next/image'

export function AppFooter() {
  return (
    <footer className="px-4 py-4 border-t border-white/10 bg-black/40 backdrop-blur-sm">
      <div className="max-w-6xl mx-auto flex items-center justify-between text-xs text-slate-500 font-mono">
        <div className="flex items-center gap-2">
          <Image
            src="/logo.png"
            alt="Trellis"
            width={240}
            height={80}
            className="h-20 w-auto -my-8 invert mix-blend-screen opacity-50"
          />
          <span className="hidden sm:inline">— Solana Devnet</span>
        </div>
        <div className="flex items-center gap-6">
          <span className="opacity-50">&copy; {new Date().getFullYear()} Trellis.</span>
        </div>
      </div>
    </footer>
  )
}
