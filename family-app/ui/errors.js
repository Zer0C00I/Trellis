// Every failed command rejects with an ErrorDto from Rust:
// { kind, message, details? }. Anything else is wrapped so screens can rely
// on that shape.
const DEFAULT_MESSAGE = 'Что-то пошло не так. Попробуйте ещё раз.';

function safeString(v) {
  try {
    return String(v);
  } catch {
    return 'unknown error';
  }
}

function normalize(e) {
  if (
    e && typeof e === 'object' &&
    typeof e.kind === 'string' && typeof e.message === 'string' && e.message !== ''
  ) {
    if (e.details == null || typeof e.details === 'string') return e;
    return { kind: e.kind, message: e.message, details: safeString(e.details) };
  }
  let details;
  try {
    details = e && typeof e === 'object' && 'message' in e ? safeString(e.message) : safeString(e);
  } catch {
    details = 'unknown error';
  }
  return { kind: 'internal', message: DEFAULT_MESSAGE, details };
}

export function normalizeError(e) {
  try {
    return normalize(e);
  } catch {
    return { kind: 'internal', message: DEFAULT_MESSAGE, details: 'unknown error' };
  }
}
