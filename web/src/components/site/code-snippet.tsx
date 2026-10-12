'use client'

import { useState } from 'react'
import { Check, Copy } from 'lucide-react'

const CODE = `// programs/family-vault/src/state.rs
#[account]
pub struct Vault {
    pub owner: Pubkey,
    pub vault_token_account: Pubkey,
    pub daily_limit: u64,
    pub spent_today: u64,
    pub last_reset_day: i64,
    pub is_pool: bool,
}`

const K = ({ children }: { children: React.ReactNode }) => <span className="text-bloom">{children}</span>
const T = ({ children }: { children: React.ReactNode }) => <span className="text-[#b9d8c1]">{children}</span>

export function CodeSnippet() {
  const [copied, setCopied] = useState(false)

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(CODE)
      setCopied(true)
      setTimeout(() => setCopied(false), 1500)
    } catch {
      /* clipboard blocked; the code is selectable */
    }
  }

  const field = (name: string, type: string) => (
    <>
      {'    '}
      <K>pub</K> {name}: <T>{type}</T>,{'\n'}
    </>
  )

  return (
    <div className="relative overflow-hidden rounded-[14px] bg-ivy">
      <button
        type="button"
        onClick={copy}
        className="absolute right-3 top-3 inline-flex h-9 items-center gap-1.5 rounded-md px-3 text-sm text-lichen/75 transition-colors hover:bg-lichen/10 hover:text-lichen"
      >
        {copied ? <Check className="size-4" /> : <Copy className="size-4" />}
        {copied ? 'Copied' : 'Copy'}
      </button>
      <pre className="overflow-x-auto p-6 pt-12 font-mono text-[13.5px] leading-relaxed text-lichen sm:pt-6">
        <span className="text-lichen/50">{'// programs/family-vault/src/state.rs'}</span>
        {'\n'}
        <K>#[account]</K>
        {'\n'}
        <K>pub struct</K> <T>Vault</T> {'{'}
        {'\n'}
        {field('owner', 'Pubkey')}
        {field('vault_token_account', 'Pubkey')}
        {field('daily_limit', 'u64')}
        {field('spent_today', 'u64')}
        {field('last_reset_day', 'i64')}
        {field('is_pool', 'bool')}
        {'}'}
      </pre>
    </div>
  )
}
