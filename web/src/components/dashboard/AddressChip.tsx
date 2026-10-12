'use client'

import { useState } from 'react'
import { Check, Copy, ExternalLink } from 'lucide-react'
import { explorerAddressUrl } from '@/config/solana'

export function shortAddress(address: string) {
  return `${address.slice(0, 4)}…${address.slice(-4)}`
}

/** A short wallet address with copy and, optionally, an explorer link. */
export function AddressChip({
  address,
  explorer = false,
  tone = 'light',
}: {
  address: string
  explorer?: boolean
  tone?: 'light' | 'dark'
}) {
  const [copied, setCopied] = useState(false)
  const muted =
    tone === 'dark'
      ? 'text-lichen/70 hover:text-lichen hover:bg-lichen/10'
      : 'text-moss hover:text-ivy hover:bg-ivy/[0.06]'

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(address)
      setCopied(true)
      setTimeout(() => setCopied(false), 1500)
    } catch {
      /* clipboard blocked: the full address is in the title attribute */
    }
  }

  return (
    <span className="inline-flex items-center gap-0.5">
      <span title={address} className={`font-mono text-[13px] ${tone === 'dark' ? 'text-lichen/80' : 'text-moss'}`}>
        {shortAddress(address)}
      </span>
      <button
        type="button"
        onClick={copy}
        className={`grid size-8 place-items-center rounded-md transition-colors ${muted}`}
        aria-label={copied ? 'Address copied' : 'Copy address'}
      >
        {copied ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
      </button>
      {explorer && (
        <a
          href={explorerAddressUrl(address)}
          target="_blank"
          rel="noopener noreferrer"
          className={`grid size-8 place-items-center rounded-md transition-colors ${muted}`}
          aria-label="Open in Solana Explorer"
        >
          <ExternalLink className="size-3.5" />
        </a>
      )}
    </span>
  )
}
