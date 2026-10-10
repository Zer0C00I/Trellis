import { Check } from 'lucide-react'

type Step = { label: string; done: boolean; action?: { label: string; onClick: () => void } }

/** What's left before the vault is useful. Hidden once everything is done. */
export function SetupChecklist({ steps }: { steps: Step[] }) {
  const remaining = steps.filter((s) => !s.done).length
  if (remaining === 0) return null

  return (
    <section aria-labelledby="setup-heading" className="rounded-[14px] border border-rule bg-paper p-5 sm:p-6">
      <div className="flex items-baseline justify-between gap-4">
        <h2 id="setup-heading" className="text-[19px] font-semibold">
          Finish setting up
        </h2>
        <span className="tabular text-sm text-moss">
          {steps.length - remaining} of {steps.length} done
        </span>
      </div>
      <ol className="mt-4 space-y-1">
        {steps.map((s) => (
          <li key={s.label} className="flex min-h-11 items-center gap-3">
            <span
              aria-hidden
              className={`grid size-6 shrink-0 place-items-center rounded-full border-2 ${
                s.done ? 'border-sap bg-sap text-white' : 'border-ivy/30'
              }`}
            >
              {s.done && <Check className="size-3.5" strokeWidth={3} />}
            </span>
            <span className={`flex-1 text-[15px] ${s.done ? 'text-moss line-through decoration-moss/40' : ''}`}>
              {s.label}
              <span className="sr-only">{s.done ? ' (done)' : ' (to do)'}</span>
            </span>
            {!s.done && s.action && (
              <button
                type="button"
                onClick={s.action.onClick}
                className="rounded-md px-2 py-1 text-[15px] font-medium text-wisteria underline decoration-wisteria/40 underline-offset-4 hover:decoration-wisteria"
              >
                {s.action.label}
              </button>
            )}
          </li>
        ))}
      </ol>
    </section>
  )
}
