import { RULES } from '@/lib/rules'

// A garden trellis: an arched frame with diagonal slats. A wisteria vine grows
// up it, and each blossom is one rule the program enforces. The drawing runs
// once on load (CSS in globals.css) and is static under reduced motion.

const W = 560
const H = 580
const ARCH = `M70 ${H - 20} V262 A210 210 0 0 1 490 262 V${H - 20} Z`

const slats: { x1: number; y1: number; x2: number; y2: number }[] = []
for (let k = -H; k < W + H; k += 84) {
  slats.push({ x1: k, y1: 0, x2: k + H, y2: H })
  slats.push({ x1: k, y1: H, x2: k + H, y2: 0 })
}

const VINE = 'M118 560 C 96 470, 196 452, 204 372 S 138 250, 250 196 S 400 150, 392 92'
const BRANCH = 'M262 190 C 330 196, 400 236, 424 300'

const leaves = [
  { x: 132, y: 500, r: -35 },
  { x: 188, y: 430, r: 30 },
  { x: 176, y: 300, r: -20 },
  { x: 232, y: 214, r: 40 },
  { x: 330, y: 168, r: -30 },
  { x: 372, y: 228, r: 25 },
  { x: 404, y: 262, r: -40 },
  { x: 384, y: 118, r: 20 },
]

const RACEME = [
  { dx: 0, dy: 0, r: 9.5 },
  { dx: -7, dy: 15, r: 9 },
  { dx: 6, dy: 28, r: 8.2 },
  { dx: -4, dy: 41, r: 7.2 },
  { dx: 5, dy: 53, r: 6.2 },
  { dx: -1, dy: 63, r: 5 },
  { dx: 2, dy: 72, r: 3.8 },
]
const PETALS = ['#5b4aa0', '#8676c6', '#b3a7e0', '#7a69bd', '#c9bfea', '#9a8cd4', '#dcd5f0']

// `label` is where the label's top-left (or top-right, for side 'left') sits, in drawing units.
const blossoms = [
  {
    x: 204,
    y: 372,
    delay: 1.35,
    text: `New names wait ${RULES.beneficiaryWaitHours}\u00a0hours before they can be paid`,
    side: 'left' as const,
    label: { x: 186, y: 420 },
  },
  {
    x: 300,
    y: 178,
    delay: 1.7,
    text: `${RULES.dailyLimitUsdc}\u00a0USDC can leave per day`,
    side: 'right' as const,
    label: { x: 322, y: 196 },
  },
  {
    x: 424,
    y: 300,
    delay: 2.0,
    text: 'Above that, a guardian signs',
    side: 'left' as const,
    label: { x: 416, y: 386 },
  },
]

function Raceme({ x, y, delay }: { x: number; y: number; delay: number }) {
  return (
    <g className="blossom" style={{ ['--d' as string]: `${delay}s` }}>
      <path d={`M${x} ${y - 6} q 2 4 0 8`} stroke="#2f6b40" strokeWidth="2" fill="none" />
      {RACEME.map((p, i) => (
        <circle key={i} cx={x + p.dx} cy={y + p.dy} r={p.r} fill={PETALS[i]} />
      ))}
    </g>
  )
}

export function LatticeHero() {
  return (
    <figure className="relative mx-auto w-full max-w-[520px]">
      <svg viewBox={`0 0 ${W} ${H}`} className="block h-auto w-full" role="img" aria-labelledby="lattice-title">
        <title id="lattice-title">
          A garden trellis with a wisteria vine. Each blossom is one rule the vault enforces.
        </title>
        <defs>
          <clipPath id="arch">
            <path d={ARCH} />
          </clipPath>
        </defs>

        <path d={ARCH} fill="#f5f7f2" />
        <g clipPath="url(#arch)" stroke="#17301f" strokeOpacity="0.2" strokeWidth="9">
          {slats.map((s, i) => (
            <line key={i} {...s} pathLength={1} className="lattice-slat" style={{ ['--i' as string]: i % 24 }} />
          ))}
        </g>
        <path d={ARCH} fill="none" stroke="#17301f" strokeWidth="7" strokeLinejoin="round" />
        <line x1="40" y1={H - 18} x2="520" y2={H - 18} stroke="#17301f" strokeWidth="3" strokeLinecap="round" />

        <path
          d={VINE}
          pathLength={1}
          className="vine-path"
          fill="none"
          stroke="#2f6b40"
          strokeWidth="4"
          strokeLinecap="round"
        />
        <path
          d={BRANCH}
          pathLength={1}
          className="vine-path"
          fill="none"
          stroke="#2f6b40"
          strokeWidth="3"
          strokeLinecap="round"
        />

        {leaves.map((l, i) => (
          <ellipse
            key={i}
            cx={l.x}
            cy={l.y}
            rx="11"
            ry="5"
            fill="#2f6b40"
            transform={`rotate(${l.r} ${l.x} ${l.y})`}
            className="blossom"
            style={{ ['--d' as string]: `${0.9 + i * 0.09}s` }}
          />
        ))}

        {blossoms.map((b) => (
          <Raceme key={b.text} x={b.x} y={b.y} delay={b.delay} />
        ))}
      </svg>

      {/* Labels: beside each blossom on wide screens, listed under the drawing on narrow ones. */}
      <figcaption>
        <ul className="mt-4 space-y-2 sm:mt-0">
          {blossoms.map((b) => (
            <li
              key={b.text}
              className="blossom-label flex items-center gap-2 text-[15px] text-ivy sm:absolute sm:w-max sm:max-w-[190px] sm:rounded-md sm:border sm:border-rule sm:bg-paper sm:px-3 sm:py-2 sm:text-sm sm:leading-snug sm:shadow-[0_6px_18px_-12px_rgb(23_48_31/0.5)]"
              style={{
                ['--d' as string]: `${b.delay}s`,
                top: `${(b.label.y / H) * 100}%`,
                ...(b.side === 'left'
                  ? { right: `${(1 - b.label.x / W) * 100}%` }
                  : { left: `${(b.label.x / W) * 100}%` }),
              }}
            >
              <span aria-hidden className="size-2.5 shrink-0 rounded-full bg-wisteria sm:hidden" />
              {b.text}
            </li>
          ))}
        </ul>
      </figcaption>
    </figure>
  )
}
