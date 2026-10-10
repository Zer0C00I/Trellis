'use client'

import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { useState } from 'react'
import { Menu, X } from 'lucide-react'
import { Wordmark } from './wordmark'

const LINKS = [
  { label: 'Features', href: '/features' },
  { label: 'Kiosk', href: '/hardware' },
  { label: 'Pricing', href: '/pricing' },
  { label: 'Docs', href: '/docs' },
]

export function SiteHeader() {
  const pathname = usePathname()
  const [open, setOpen] = useState(false)

  return (
    <header className="sticky top-0 z-40 border-b border-rule bg-lichen/90 backdrop-blur-md">
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-3 focus:z-50 focus:rounded-md focus:bg-ivy focus:px-3 focus:py-2 focus:text-lichen"
      >
        Skip to content
      </a>
      <div className="mx-auto flex h-16 max-w-[1200px] items-center gap-8 px-4 sm:px-6">
        <Link href="/" className="-my-2 shrink-0" aria-label="Trellis home">
          <Wordmark className="h-12 w-auto" />
        </Link>

        <nav aria-label="Main" className="hidden md:block">
          <ul className="flex items-center gap-1">
            {LINKS.map(({ label, href }) => {
              const current = pathname.startsWith(href)
              return (
                <li key={href}>
                  <Link
                    href={href}
                    aria-current={current ? 'page' : undefined}
                    className={`rounded-md px-3 py-2 text-[15px] transition-colors hover:bg-ivy/[0.06] ${
                      current ? 'font-semibold text-ivy' : 'text-moss hover:text-ivy'
                    }`}
                  >
                    {label}
                  </Link>
                </li>
              )
            })}
          </ul>
        </nav>

        <div className="ml-auto flex items-center gap-2">
          <Link
            href="/dashboard"
            className="hidden h-10 items-center rounded-md bg-ivy px-4 text-[15px] font-medium text-lichen transition-colors hover:bg-ivy-2 sm:inline-flex"
          >
            Open dashboard
          </Link>
          <button
            type="button"
            className="grid size-10 place-items-center rounded-md text-ivy hover:bg-ivy/[0.06] md:hidden"
            aria-expanded={open}
            aria-controls="mobile-nav"
            aria-label={open ? 'Close menu' : 'Open menu'}
            onClick={() => setOpen((o) => !o)}
          >
            {open ? <X className="size-5" /> : <Menu className="size-5" />}
          </button>
        </div>
      </div>

      {open && (
        <nav id="mobile-nav" aria-label="Main" className="border-t border-rule px-4 pb-5 pt-2 md:hidden">
          <ul className="flex flex-col">
            {LINKS.map(({ label, href }) => (
              <li key={href}>
                <Link
                  href={href}
                  onClick={() => setOpen(false)}
                  className="block border-b border-rule py-3 text-lg text-ivy"
                >
                  {label}
                </Link>
              </li>
            ))}
          </ul>
          <Link
            href="/dashboard"
            className="mt-4 flex h-12 items-center justify-center rounded-md bg-ivy text-base font-medium text-lichen"
          >
            Open dashboard
          </Link>
        </nav>
      )}
    </header>
  )
}
