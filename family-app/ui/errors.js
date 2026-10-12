// Every failed command rejects with an ErrorDto from Rust:
// { kind, key, params, details? }, where `key` and `params` name a catalog
// sentence (see i18n.js). Anything else is wrapped so screens can rely on
// that shape.
const GENERIC = 'error.generic';

function safeString(v) {
  try {
    return String(v);
  } catch {
    return 'unknown error';
  }
}

function isPlainObject(v) {
  return v !== null && typeof v === 'object' && !Array.isArray(v);
}

function normalize(e) {
  if (isPlainObject(e) && typeof e.kind === 'string' && typeof e.key === 'string' && e.key !== '') {
    const params = isPlainObject(e.params) ? e.params : {};
    if (e.details == null || typeof e.details === 'string') return { ...e, params };
    return { kind: e.kind, key: e.key, params, details: safeString(e.details) };
  }
  let details;
  try {
    details = e && typeof e === 'object' && 'message' in e ? safeString(e.message) : safeString(e);
  } catch {
    details = 'unknown error';
  }
  return { kind: 'internal', key: GENERIC, params: {}, details };
}

export function normalizeError(e) {
  try {
    return normalize(e);
  } catch {
    return { kind: 'internal', key: GENERIC, params: {}, details: 'unknown error' };
  }
}
