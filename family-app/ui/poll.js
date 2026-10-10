// How long to wait before the next look for a vault that is still being
// created for us: patient but not hammering the network forever.
export function waitingDelayMs(attempt) {
  const n = Number.isNaN(Number(attempt)) || attempt == null ? 0 : Math.floor(Number(attempt));
  if (n < 6) return 5000;
  if (n < 12) return 10000;
  return 30000;
}
