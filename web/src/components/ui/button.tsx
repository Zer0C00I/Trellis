import * as React from 'react'
import { Slot } from '@radix-ui/react-slot'
import { cva, type VariantProps } from 'class-variance-authority'

import { cn } from '@/lib/utils'

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-[15px] font-medium transition-colors disabled:pointer-events-none disabled:opacity-45 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 shrink-0 [&_svg]:shrink-0 outline-none focus-visible:outline-3 focus-visible:outline-wisteria focus-visible:outline-offset-2 aria-invalid:border-destructive",
  {
    variants: {
      variant: {
        default: 'bg-wisteria text-white hover:bg-wisteria-ink',
        destructive: 'bg-rust text-white hover:bg-rust/90',
        outline: 'border border-ivy/35 bg-transparent text-ivy hover:bg-ivy/[0.06] hover:border-ivy/60',
        secondary: 'bg-ivy text-lichen hover:bg-ivy-2',
        ghost: 'text-ivy hover:bg-ivy/[0.06]',
        link: 'text-wisteria underline underline-offset-4 decoration-wisteria/40 hover:decoration-wisteria',
      },
      size: {
        default: 'h-10 px-4 has-[>svg]:px-3',
        sm: 'h-9 gap-1.5 px-3 text-sm has-[>svg]:px-2.5',
        lg: 'h-12 px-6 text-base has-[>svg]:px-5',
        icon: 'size-10',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  },
)

function Button({
  className,
  variant,
  size,
  asChild = false,
  ...props
}: React.ComponentProps<'button'> &
  VariantProps<typeof buttonVariants> & {
    asChild?: boolean
  }) {
  const Comp = asChild ? Slot : 'button'

  return <Comp data-slot="button" className={cn(buttonVariants({ variant, size, className }))} {...props} />
}

export { Button, buttonVariants }
