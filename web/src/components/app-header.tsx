'use client'
import { usePathname } from 'next/navigation'
import { useState } from 'react'
import Link from 'next/link'
import { Menu, X } from 'lucide-react'
import { ClusterUiSelect } from './cluster/cluster-ui'
import { WalletButton } from '@/components/solana/wallet-button'
import { Wordmark } from '@/components/site/wordmark'

export function AppHeader({ links = [] }: { links: { label: string; path: string }[] }) {
  const pathname = usePathname()
  const [showMenu, setShowMenu] = useState(false)

  function isActive(path: string) {
    return path === '/' ? pathname === '/' : pathname.startsWith(path)
  }

  return (
    <header className="sticky top-0 z-40 border-b border-rule bg-lichen/90 backdrop-blur-md">
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-3 focus:z-50 focus:rounded-md focus:bg-ivy focus:px-3 focus:py-2 focus:text-lichen"
      >
        Skip to content
      </a>
      <div className="mx-auto flex h-16 max-w-[1200px] items-center gap-6 px-4 sm:px-6">
        <Link className="-my-2 shrink-0" href="/" aria-label="Trellis home">
          <Wordmark className="h-12 w-auto" />
        </Link>
        <nav aria-label="Dashboard" className="hidden md:block">
          <ul className="flex items-center gap-1">
            {links.map(({ label, path }) => (
              <li key={path}>
                <Link
                  className={`rounded-md px-3 py-2 text-[15px] transition-colors hover:bg-ivy/[0.06] ${
                    isActive(path) ? 'font-semibold text-ivy' : 'text-moss hover:text-ivy'
                  }`}
                  aria-current={isActive(path) ? 'page' : undefined}
                  href={path}
                >
                  {label}
                </Link>
              </li>
            ))}
          </ul>
        </nav>

        <div className="ml-auto hidden items-center gap-2 md:flex">
          <ClusterUiSelect />
          <WalletButton />
        </div>

        <button
          type="button"
          className="ml-auto grid size-10 place-items-center rounded-md text-ivy hover:bg-ivy/[0.06] md:hidden"
          aria-expanded={showMenu}
          aria-label={showMenu ? 'Close menu' : 'Open menu'}
          onClick={() => setShowMenu(!showMenu)}
        >
          {showMenu ? <X className="size-5" /> : <Menu className="size-5" />}
        </button>
      </div>

      {showMenu && (
        <div className="border-t border-rule px-4 pb-5 pt-2 md:hidden">
          <ul className="flex flex-col">
            {links.map(({ label, path }) => (
              <li key={path}>
                <Link
                  className={`block border-b border-rule py-3 text-lg ${isActive(path) ? 'font-semibold text-ivy' : 'text-moss'}`}
                  href={path}
                  onClick={() => setShowMenu(false)}
                >
                  {label}
                </Link>
              </li>
            ))}
          </ul>
          <div className="mt-4 flex flex-wrap items-center gap-2">
            <WalletButton />
            <ClusterUiSelect />
          </div>
        </div>
      )}
    </header>
  )
}
