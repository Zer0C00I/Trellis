// The only place that talks to the Rust side. Outside Tauri (a plain browser,
// or "?mock=..." in the URL) it falls back to a scripted fake so every screen
// can be looked at without a device or a network.
import { normalizeError } from './errors.js';

const tauri = globalThis.__TAURI__?.core;
// Inside the real app "?mock" is ignored: a stray URL must never swap the
// real backend for a fake one that shows invented balances.
const inTauri = Boolean(globalThis.__TAURI__ || globalThis.__TAURI_INTERNALS__);
const wantsMock = !inTauri && new URLSearchParams(globalThis.location?.search ?? '').has('mock');
export const isMock = !tauri || wantsMock;

let backend;

async function getBackend() {
  if (!backend) {
    backend = isMock ? (await import('./mock-api.js')).default : { invoke: tauri.invoke };
  }
  return backend;
}

export async function call(command, args = {}) {
  const b = await getBackend();
  try {
    return await b.invoke(command, args);
  } catch (e) {
    throw normalizeError(e);
  }
}
