// The kiosk's "My wallet" screen as built in kiosk-gui/ui: black on white for
// WCAG 2.2 AAA contrast, family as large black buttons, and the controls that
// stay put on every screen in the right column. Drawn at the device's 800x480
// proportions; purely illustrative.

const FAMILY = ['Grandson Sasha', 'Daughter Masha', 'Son Ivan']

export function KioskDevice({ className = '' }: { className?: string }) {
  return (
    <div className={`w-full max-w-[560px] ${className}`} aria-hidden>
      <div className="rounded-[22px] bg-[#0d1a11] p-3 shadow-[0_30px_60px_-30px_rgb(0_0_0/0.6)] ring-1 ring-black/40">
        <div className="grid aspect-[800/480] grid-cols-[1fr_27%] gap-[2.5%] overflow-hidden rounded-[10px] bg-white p-[2.5%] text-black">
          <div className="flex flex-col gap-[3%]">
            <span className="text-[clamp(10px,2.2vw,18px)] leading-none">My wallet</span>
            <span className="tabular text-[clamp(12px,2.6vw,22px)] font-semibold leading-none">120 USDC</span>
            {FAMILY.map((name) => (
              <span
                key={name}
                className="flex flex-1 items-center justify-center rounded-[4px] bg-black text-[clamp(8px,1.6vw,13px)] font-medium text-white"
              >
                {name}
              </span>
            ))}
          </div>
          <div className="flex flex-col justify-between">
            <span className="rounded-[4px] border-2 border-black py-[12%] text-center text-[clamp(7px,1.3vw,11px)] font-medium">
              Family wallet
            </span>
            <span className="self-end rounded-[4px] border-2 border-black px-[18%] py-[8%] text-[clamp(10px,2vw,16px)] font-semibold">
              ?
            </span>
          </div>
        </div>
      </div>
      <div className="mx-auto h-3 w-[34%] rounded-b-[10px] bg-[#0d1a11]" />
    </div>
  )
}
