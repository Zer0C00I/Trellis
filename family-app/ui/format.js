// Dates only. Money is formatted by Rust; JavaScript never does money math.
import { currentLocale, t } from './i18n.js';

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

// "today, 14:32", "Oct 5, 09:05", "Dec 31, 2025, 23:59" (the month and the
// order follow the current language); '' if unknown.
export function formatTime(unixSeconds, nowMs = Date.now(), timeZone = undefined, locale = currentLocale()) {
  if (typeof unixSeconds !== 'number' || !Number.isFinite(unixSeconds) || Math.abs(unixSeconds) >= 1e11) {
    return '';
  }
  if (typeof nowMs !== 'number' || !Number.isFinite(nowMs)) nowMs = Date.now();
  try {
    const t0 = parts(unixSeconds * 1000, timeZone);
    const n = parts(nowMs, timeZone);
    const clock = `${t0.hour}:${t0.minute}`;
    if (t0.year === n.year && t0.month === n.month && t0.day === n.day) return t('time.today', { time: clock });
    const date = new Intl.DateTimeFormat(locale, { timeZone, day: 'numeric', month: 'short' }).format(unixSeconds * 1000);
    return t0.year === n.year
      ? t('time.this_year', { date, time: clock })
      : t('time.other_year', { date, year: t0.year, time: clock });
  } catch {
    return '';
  }
}
