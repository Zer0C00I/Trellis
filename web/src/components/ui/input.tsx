import * as React from 'react'

import { cn } from '@/lib/utils'

function Input({ className, type, ...props }: React.ComponentProps<'input'>) {
  return (
    <input
      type={type}
      data-slot="input"
      className={cn(
        'flex h-11 w-full min-w-0 rounded-md border border-input bg-paper px-3 py-1 text-base text-ivy transition-[border-color,box-shadow] outline-none placeholder:text-moss/70 disabled:cursor-not-allowed disabled:opacity-50',
        'focus-visible:border-wisteria focus-visible:ring-[3px] focus-visible:ring-wisteria/25 focus-visible:outline-none',
        'aria-invalid:border-rust aria-invalid:ring-rust/15',
        className,
      )}
      {...props}
    />
  )
}

export { Input }
