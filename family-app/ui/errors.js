// Every failed command rejects with an ErrorDto from Rust:
// { kind, message, details? }. Anything else is wrapped so screens can rely
// on that shape.
export function normalizeError(e) {
  if (e && typeof e === 'object' && typeof e.kind === 'string' && typeof e.message === 'string') {
    return e;
  }
  const details = e && typeof e === 'object' && 'message' in e ? String(e.message) : String(e);
  return { kind: 'internal', message: 'Что-то пошло не так. Попробуйте ещё раз.', details };
}
