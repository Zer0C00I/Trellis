'use client'

import { Toaster as Sonner, ToasterProps } from 'sonner'

const Toaster = ({ ...props }: ToasterProps) => {
  return (
    <Sonner
      theme="light"
      position="bottom-right"
      className="toaster group"
      toastOptions={{
        classNames: {
          toast: '!rounded-[10px] !border-rule !font-sans !text-[15px] !shadow-[0_12px_32px_-14px_rgb(23_48_31/0.4)]',
          description: '!text-moss',
          actionButton: '!bg-ivy !text-lichen !rounded-md',
        },
      }}
      style={
        {
          '--normal-bg': 'var(--paper)',
          '--normal-text': 'var(--ivy)',
          '--normal-border': 'var(--rule)',
          '--success-bg': 'var(--paper)',
          '--success-text': 'var(--sap)',
          '--error-bg': 'var(--paper)',
          '--error-text': 'var(--rust)',
        } as React.CSSProperties
      }
      {...props}
    />
  )
}

export { Toaster }
