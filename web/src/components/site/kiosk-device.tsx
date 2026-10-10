// The kiosk's "My wallet" screen as redesigned in kiosk-gui/ui: an Ivy panel
// with the balance on the left and the family as large tiles on the right.
// Drawn at the device's 800x480 proportions; purely illustrative.

const FAMILY = ['Внук Саша', 'Дочка Маша', 'Сын Иван', 'Соседка Нина']

export function KioskDevice({ className = '' }: { className?: string }) {
  return (
    <div className={`w-full max-w-[560px] ${className}`} aria-hidden>
      <div className="rounded-[22px] bg-[#0d1a11] p-3 shadow-[0_30px_60px_-30px_rgb(0_0_0/0.6)] ring-1 ring-black/40">
        <div className="grid aspect-[800/480] grid-cols-[36%_1fr] gap-[2.5%] overflow-hidden rounded-[10px] bg-lichen p-[2.5%]">
          <div className="flex flex-col rounded-[6px] bg-ivy p-[7%] text-lichen">
            <span className="text-[clamp(8px,1.6vw,13px)] opacity-80">Мой кошелёк</span>
            <span className="tabular mt-[4%] text-[clamp(14px,3.4vw,28px)] font-semibold leading-none">$1 240</span>
            <span className="mt-[3%] text-[clamp(7px,1.3vw,11px)] opacity-70">USDC</span>
            <span className="mt-auto rounded-[4px] border border-lichen/50 py-[5%] text-center text-[clamp(7px,1.3vw,11px)]">
              Семейный кошелёк
            </span>
          </div>
          <div className="grid grid-cols-2 content-start gap-[4%]">
            {FAMILY.map((name) => (
              <span
                key={name}
                className="flex aspect-[2.6/1] items-center rounded-[5px] border-l-[3px] border-wisteria bg-paper px-[8%] text-[clamp(8px,1.6vw,13px)] font-medium text-ivy shadow-[0_1px_0_rgb(23_48_31/0.12)]"
              >
                {name}
              </span>
            ))}
          </div>
        </div>
      </div>
      <div className="mx-auto h-3 w-[34%] rounded-b-[10px] bg-[#0d1a11]" />
    </div>
  )
}
