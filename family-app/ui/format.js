// Dates only. Money is formatted by Rust; JavaScript never does money math.
const MONTHS = ['янв.', 'февр.', 'мар.', 'апр.', 'мая', 'июн.', 'июл.', 'авг.', 'сент.', 'окт.', 'нояб.', 'дек.'];

function parts(ms, timeZone) {
  const f = new Intl.DateTimeFormat('en-GB', {
    timeZone,
    year: 'numeric',
    month: 'numeric',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    hourCycle: 'h23',
  });
  const out = {};
  for (const p of f.formatToParts(new Date(ms))) out[p.type] = p.value;
  return out;
}

// "сегодня, 14:32", "5 окт., 09:05", "5 окт. 2025, 09:05"; '' if unknown.
export function formatTime(unixSeconds, nowMs = Date.now(), timeZone = undefined) {
  if (typeof unixSeconds !== 'number' || !Number.isFinite(unixSeconds) || Math.abs(unixSeconds) >= 1e11) {
    return '';
  }
  if (typeof nowMs !== 'number' || !Number.isFinite(nowMs)) nowMs = Date.now();
  try {
    const t = parts(unixSeconds * 1000, timeZone);
    const n = parts(nowMs, timeZone);
    const clock = `${t.hour}:${t.minute}`;
    if (t.year === n.year && t.month === n.month && t.day === n.day) return `сегодня, ${clock}`;
    const date = `${Number(t.day)} ${MONTHS[Number(t.month) - 1]}`;
    return t.year === n.year ? `${date}, ${clock}` : `${date} ${t.year}, ${clock}`;
  } catch {
    return '';
  }
}
