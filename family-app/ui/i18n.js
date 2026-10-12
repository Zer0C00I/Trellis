// Translations. Rust sends keys and parameters, never sentences; this module
// turns them into text in the current language. No DOM here, so it is tested
// in Node.
import en from './locales/en.js';
import ru from './locales/ru.js';

export const LOCALES = ['en', 'ru'];
const CATALOGS = { en, ru };
const STORAGE_KEY = 'trellis.locale';

let locale = 'en';

// Replaceable so a test can turn a missing key into a failure.
export const hooks = {
  onMissing: (key) => console.warn(`[i18n] missing translation: ${key}`),
};

export function currentLocale() {
  return locale;
}

export function setLocale(next, { persist = true } = {}) {
  if (!LOCALES.includes(next)) return;
  locale = next;
  if (globalThis.document?.documentElement) globalThis.document.documentElement.lang = next;
  if (!persist) return;
  try {
    globalThis.localStorage?.setItem(STORAGE_KEY, next);
  } catch {
    /* storage may be blocked; the choice then lasts until the app is closed */
  }
}

// The saved choice, else the system language (`ru` or `en`), else English.
export function detectLocale({ stored, navigatorLanguage } = {}) {
  if (LOCALES.includes(stored)) return stored;
  const prefix = String(navigatorLanguage ?? '').toLowerCase().split(/[-_]/)[0];
  return LOCALES.includes(prefix) ? prefix : 'en';
}

export function initLocale() {
  let stored = null;
  try {
    stored = globalThis.localStorage?.getItem(STORAGE_KEY) ?? null;
  } catch {
    /* no storage: fall back to the system language */
  }
  setLocale(detectLocale({ stored, navigatorLanguage: globalThis.navigator?.language }), { persist: false });
  return locale;
}

function lookup(key) {
  return CATALOGS[locale][key] ?? CATALOGS.en[key];
}

// A parameter may itself be a message from Rust, { key, params }.
function paramText(value) {
  if (value && typeof value === 'object' && typeof value.key === 'string') return t(value.key, value.params);
  return String(value);
}

function pick(entry, params) {
  if (typeof entry !== 'object' || entry === null) return entry;
  const category = new Intl.PluralRules(locale).select(Number(params?.n));
  return entry[category] ?? entry.other;
}

// The template of `key` split around its `{name}` placeholders:
// [{ text }, { param: 'left', text: '70 USDC' }, ...].
export function tParts(key, params = {}) {
  const entry = lookup(key);
  if (entry === undefined) {
    hooks.onMissing(key);
    return [{ text: key }];
  }
  const template = pick(entry, params);
  const parts = [];
  let last = 0;
  for (const match of template.matchAll(/\{(\w+)\}/g)) {
    if (match.index > last) parts.push({ text: template.slice(last, match.index) });
    const name = match[1];
    parts.push(name in params ? { param: name, text: paramText(params[name]) } : { text: match[0] });
    last = match.index + match[0].length;
  }
  if (last < template.length) parts.push({ text: template.slice(last) });
  return parts;
}

export function t(key, params = {}) {
  return tParts(key, params).map((p) => p.text).join('');
}

// Text for a message sent by Rust ({ key, params }).
export function tMsg(message) {
  return t(message.key, message.params);
}

export function catalog(name) {
  return CATALOGS[name];
}
