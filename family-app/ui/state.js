// One shared state object; screens read it, `update` notifies the shell.
export const store = {
  info: null, // { address, address_short, network }
  wallet: null, // { sol, low_sol }
  vaults: [], // [{ address, address_short, mine }]
  selected: null, // address of the vault on screen
  vault: null, // VaultDto
  history: [], // [{ signature, key, params, time }]
  pending: null, // { what: { key, params }, signature } while an operation's outcome is unknown
  notice: null, // { key, params } of a message still on screen
  noticeKind: 'ok', // 'ok' | 'info' | 'warn'
  pairHint: false, // the person came back from a waiting that never ended
  loaded: false,
  everLoaded: false, // a refresh has succeeded at least once
  waiting: false, // first launch: the kiosk took our code, waiting for the chain
  loadError: null, // ErrorDto when the last refresh failed
};

const listeners = new Set();

export function subscribe(fn) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

export function update(patch) {
  Object.assign(store, patch);
  for (const fn of listeners) fn();
}
