// One shared state object; screens read it, `update` notifies the shell.
export const store = {
  info: null, // { address, address_short, network }
  wallet: null, // { sol, low_sol }
  vaults: [], // [{ address, address_short, mine }]
  selected: null, // address of the vault on screen
  vault: null, // VaultDto
  history: [], // [{ signature, text, time }]
  pending: null, // { what, signature } while an operation's outcome is unknown
  notice: null, // text of a success message still on screen
  loaded: false,
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
