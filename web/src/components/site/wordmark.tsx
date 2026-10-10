import Image from 'next/image'

/** The script "Trellis" mark, cropped from public/logo.png and inked in Ivy or Lichen. */
export function Wordmark({ tone = 'ivy', className = 'h-11 w-auto' }: { tone?: 'ivy' | 'lichen'; className?: string }) {
  return (
    <Image
      src={tone === 'ivy' ? '/logo-ivy.png' : '/logo-lichen.png'}
      alt="Trellis"
      width={313}
      height={244}
      priority
      className={className}
    />
  )
}
