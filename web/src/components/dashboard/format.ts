import { BN } from '@coral-xyz/anchor'
import { USDC_DECIMALS } from '@/config/solana'

/** "$1,240.00" */
export function formatUsdc(units: BN): string {
  const s = units.toString().padStart(USDC_DECIMALS + 1, '0')
  const whole = s.slice(0, -USDC_DECIMALS)
  const frac = s.slice(-USDC_DECIMALS).slice(0, 2)
  return `$${Number(whole).toLocaleString('en-US')}.${frac}`
}

/** "100 USDC", or "100.5 USDC" when there is a fractional part. */
export function formatUsdcAmount(units: BN): string {
  const s = units.toString().padStart(USDC_DECIMALS + 1, '0')
  const whole = Number(s.slice(0, -USDC_DECIMALS)).toLocaleString('en-US')
  const frac = s.slice(-USDC_DECIMALS).replace(/0+$/, '')
  return `${frac ? `${whole}.${frac}` : whole} USDC`
}

/** "3 hours ago", "in 31 hours", from seconds. */
export function relativeHours(deltaSecs: number): string {
  const abs = Math.abs(deltaSecs)
  if (abs < 3600) {
    const m = Math.max(1, Math.round(abs / 60))
    return deltaSecs < 0 ? `${m} min ago` : `in ${m} min`
  }
  const h = Math.round(abs / 3600)
  const unit = h === 1 ? 'hour' : 'hours'
  return deltaSecs < 0 ? `${h} ${unit} ago` : `in ${h} ${unit}`
}
