// A scripted stand-in for the Rust commands: same names, same shapes as
// src-tauri/src/dto.rs (sentences are catalog keys with params, as from Rust). Pick a scenario with ?mock=demo|owner|guardian|empty|waiting|unconfirmed|unconfirmed-failed|unconfirmed-waiting|send-unconfirmed|stuck|lowsol|offline
import { currentLocale } from './i18n.js';

const scenario = new URLSearchParams(location.search).get('mock') || 'owner';
const demo = scenario === 'demo';
// Sample people, named in the language the app is showing.
const NAMES = {
  en: { anna: 'Granddaughter Anna', peter: 'Neighbour Peter' },
  ru: { anna: 'Внучка Анна', peter: 'Сосед Пётр' },
};
const name = (who) => (NAMES[currentLocale()] ?? NAMES.en)[who];
const delay = (ms = 250) => new Promise((r) => setTimeout(r, ms));

const ME = '7Hn3mQe1dKq9vXw2YfPz8LbT5aRcUoJs4NgBxVyE6AtM';
const short = (a) => `${a.slice(0, 4)}…${a.slice(-4)}`;
const msg = (key, params = {}) => ({ key, params });

const payee = (who, address, rest) => ({ get name() { return name(who); }, address, address_short: short(address), ...rest });
const payees = [
  payee('anna', '9xQeWvG816bUx9EPfHnxZq3JdRkaLo5SyMtBpNwC2Ue', { active: true, status: msg('payee.ready') }),
  payee('peter', 'F3sA7kDmW2cLhVn6TqYbJ9uXoR1ePzG4iKaNvMy8CdBt', { active: false, status: msg('payee.wait_hours', { n: 31 }) }),
];

const guardians = [
  { address: ME, is_me: true },
  { address: 'B6nVr2KxQm8LdYt4WcEhUs1JzPa9GfTo7iXbNe3MvRcA', is_me: false },
].map((g) => ({ ...g, address_short: short(g.address) }));

let pool = {
  address: '4kTzLp9QwXy2VbN7mRcEa1JdUhGs6oFiMt3BnYvKe8Ax',
  is_pool: true,
  is_owner: true,
  is_guardian: false,
  balance: '250.5 USDC',
  decimals: 6,
  daily_limit: '100 USDC',
  remaining_today: '70 USDC',
  payees,
  guardians,
  pending: [],
  permissions: { deposit: true, send: true, add_payee: true, add_guardian: true, approve: false },
};
pool.address_short = short(pool.address);

const grandma = {
  address: 'Cw5RnE8yMq2KdLtV7XbZa3JhUp9GfoSi1TmBvNeY4AkD',
  is_pool: false,
  is_owner: false,
  is_guardian: true,
  balance: '1200 USDC',
  decimals: 6,
  daily_limit: null,
  remaining_today: null,
  payees: payees.slice(0, 1),
  guardians: guardians.slice(0, 1),
  pending: [{ nonce: 3, amount: '150 USDC', to: payees[0].address, to_short: payees[0].address_short, requested_at: Math.floor(Date.now() / 1000) - 600 }],
  permissions: { deposit: true, send: false, add_payee: false, add_guardian: false, approve: true },
};
grandma.address_short = short(grandma.address);
if (demo) {
  pool.balance = '340 USDC';
  pool.daily_limit = '30 USDC';
  pool.remaining_today = '30 USDC';
  Object.assign(grandma, { is_pool: true, balance: '500 USDC', daily_limit: '30 USDC', remaining_today: '30 USDC', payees, guardians });
  grandma.pending = [{ ...grandma.pending[0], amount: '35 USDC' }];
}
const demoSent = [];

const now = () => Math.floor(Date.now() / 1000);
const history = () => [
  ...demoSent,
  { signature: 's1', ...msg('history.deposit', { amount: '50 USDC' }), time: now() - 3600 },
  { signature: 's2', ...msg('history.withdraw', { amount: '12.5 USDC' }), time: now() - 86400 * 2 },
  { signature: 's3', ...msg('history.payee_added', { name: name('anna') }), time: now() - 86400 * 9 },
  { signature: 's4', ...msg('history.opened'), time: now() - 86400 * 40 },
];

let vaults = { demo: [], owner: [pool], guardian: [grandma], empty: [], waiting: [], unconfirmed: [pool], 'unconfirmed-failed': [pool], 'unconfirmed-waiting': [pool], stuck: [], lowsol: [pool] }[scenario] ?? [pool];
let solGot = false;
let pendingTx = scenario.startsWith('unconfirmed') ? { what: msg('op.send', { amount: '5 USDC' }), signature: 'sig123' } : null;
let waitingPolls = 0;
let moved = 0; // every deposit/send changes the balance, as on a real chain
const setBalance = () => {
  pool.balance = `${((demo ? 340 : 250.5) + moved).toFixed(2).replace(/\.?0+$/, '')} USDC`;
};

// Test hook: the chain moves on its own (someone else paid in).
globalThis.__mockBump = () => {
  moved += 1;
  setBalance();
};

const err = (kind, key, details, params = {}) => ({ kind, key, params, details: details ?? null });

const handlers = {
  app_info: () => ({ address: ME, address_short: short(ME), network: 'devnet' }),
  wallet: () => {
    if (scenario === 'offline') throw err('no_connection', 'error.no_connection');
    return !solGot && ['lowsol', 'empty', 'waiting', 'stuck'].includes(scenario) ? { sol: '0', low_sol: true } : { sol: '0.1', low_sol: false };
  },
  list_vaults: () => {
    if ((scenario === 'waiting' || demo) && ++waitingPolls >= 3) vaults = demo ? [pool, grandma] : [grandma];
    return vaults.map((v) => ({ address: v.address, address_short: v.address_short, mine: v.is_owner }));
  },
  load_vault: ({ vault }) => {
    const v = vaults.find((x) => x.address === vault);
    if (!v) throw err('rejected', 'error.rejected_generic');
    return structuredClone(v); // like the real IPC: a snapshot, not a live object
  },
  load_history: () => history(),
  pending_status: () => pendingTx,
  check_pending: () => {
    if (!pendingTx) return { status: 'none', ...msg('resolution.none') };
    if (scenario === 'unconfirmed-waiting') return { status: 'waiting', ...msg('resolution.waiting') };
    pendingTx = null;
    if (scenario === 'unconfirmed-failed') return { status: 'failed', ...msg('resolution.failed') };
    return { status: 'confirmed', ...msg('resolution.confirmed') };
  },
  deposit: () => {
    moved += 10;
    setBalance();
    return { signature: 'sigD', ...msg('notice.deposit') };
  },
  send_money: ({ amount }) => {
    if (scenario === 'send-unconfirmed') {
      pendingTx = { what: msg('op.send', { amount: `${amount} USDC` }), signature: 'sigU' };
      throw err('unconfirmed', 'error.unconfirmed');
    }
    const left = Number(String(pool.remaining_today).split(' ')[0]);
    if (Number(amount) > left) {
      return { signature: 'sigR', ...msg('notice.request_filed') };
    }
    moved -= Number(amount);
    if (demo) demoSent.unshift({ signature: `d${demoSent.length}`, ...msg('history.withdraw', { amount: `${amount} USDC` }), time: now() });
    if (demo) pool.remaining_today = `${left - Number(amount)} USDC`;
    setBalance();
    return { signature: 'sigS', ...msg('notice.sent') };
  },
  add_payee: () => ({ signature: 'sigP', ...msg('notice.payee_added') }),
  add_guardian: () => ({ signature: 'sigG', ...msg('notice.guardian_added') }),
  approve_request: () => {
    grandma.pending = [];
    grandma.permissions.approve = false;
    return { signature: 'sigA', ...msg('notice.approved') };
  },
  open_family_account: () => {
    vaults = [pool];
    return { address: pool.address, address_short: pool.address_short, mine: true };
  },
  get_test_sol: () => {
    solGot = true;
    return null;
  },
  // Tauri 2 passes JavaScript arguments in camelCase (Rust: qr_text -> qrText).
  pair_with_qr: ({ qrText }) => {
    if (!String(qrText).startsWith('fvpair1:')) throw err('invalid', 'error.pairing.bad_qr');
    return null;
  },
  pair_with_pin: ({ pin }) => {
    if (pin === '000000') throw err('rejected', 'error.pairing.write_rejected', 'att 0x0e');
    return null;
  },
};

export default {
  async invoke(command, args) {
    await delay();
    const handler = handlers[command];
    if (!handler) throw err('internal', 'error.internal', `unknown command ${command}`);
    const result = handler(args ?? {});
    // Test hook: hold back ONE answer (computed now) to make it arrive late.
    const extra = globalThis.__mockDelay?.[command];
    if (extra) {
      delete globalThis.__mockDelay[command];
      await delay(extra);
    }
    return result;
  },
};
