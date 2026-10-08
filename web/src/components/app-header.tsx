'use client'
import { usePathname } from 'next/navigation'
import { useState } from 'react'
import Link from 'next/link'
import { Button } from '@/components/ui/button'
import { Menu, X, Shield } from 'lucide-react'
import { ClusterUiSelect } from './cluster/cluster-ui'
import { WalletButton } from '@/components/solana/wallet-button'

import Image from 'next/image'

export function AppHeader({ links = [] }: { links: { label: string; path: string }[] }) {
  const pathname = usePathname()
  const [showMenu, setShowMenu] = useState(false)

  function isActive(path: string) {
    return path === '/' ? pathname === '/' : pathname.startsWith(path)
  }

  return (
    <header className="relative z-50 px-4 py-4 bg-black/40 backdrop-blur-2xl border-b border-white/10">
      <div className="mx-auto flex justify-between items-center max-w-6xl">
        <div className="flex items-center gap-4">
          <Link className="flex items-center gap-2 hover:opacity-80 transition-opacity" href="/">
            <Image src="/logo.png" alt="Trellis" width={240} height={80} className="h-20 w-auto -my-6 invert mix-blend-screen" />
          </Link>
          <div className="hidden md:flex items-center ml-4">
            <ul className="flex gap-4 flex-nowrap items-center">
              {links.map(({ label, path }) => (
                <li key={path}>
                  <Link
                    className={`text-sm hover:text-white transition-colors ${isActive(path) ? 'text-white font-medium' : 'text-muted-foreground'}`}
                    href={path}
                  >
                    {label}
                  </Link>
                </li>
              ))}
            </ul>
          </div>
        </div>

        <Button variant="ghost" size="icon" className="md:hidden" onClick={() => setShowMenu(!showMenu)}>
          {showMenu ? <X className="h-6 w-6" /> : <Menu className="h-6 w-6" />}
        </Button>

        <div className="hidden md:flex items-center gap-3">
          <WalletButton />
          <ClusterUiSelect />
        </div>

        {showMenu && (
          <div className="md:hidden fixed inset-x-0 top-[56px] bottom-0 bg-background/95 backdrop-blur-xl">
            <div className="flex flex-col p-4 gap-4 border-t border-white/5">
              <ul className="flex flex-col gap-4">
                {links.map(({ label, path }) => (
                  <li key={path}>
                    <Link
                      className={`block text-lg py-2 ${isActive(path) ? 'text-white' : 'text-muted-foreground'}`}
                      href={path}
                      onClick={() => setShowMenu(false)}
                    >
                      {label}
                    </Link>
                  </li>
                ))}
              </ul>
              <div className="flex flex-col gap-4">
                <WalletButton />
                <ClusterUiSelect />
              </div>
            </div>
          </div>
        )}
      </div>
    </header>
  )
}

