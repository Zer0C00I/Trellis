'use client'

export function AppFooter() {
  return (
    <footer className="mt-16 border-t border-rule">
      <div className="mx-auto flex max-w-[1200px] flex-col gap-1 px-4 py-6 text-sm text-moss sm:flex-row sm:justify-between sm:px-6">
        <span>Trellis on Solana Devnet. Prototype, not audited: use test funds only.</span>
        <span>&copy; {new Date().getFullYear()} Trellis</span>
      </div>
    </footer>
  )
}
