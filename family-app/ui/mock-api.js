// A scripted stand-in for the Rust commands: same names, same shapes as
// src-tauri/src/dto.rs. Pick a scenario with ?mock=owner|guardian|empty|waiting|unconfirmed|lowsol
const scenario = new URLSearchParams(location.search).get('mock') || 'owner';
const delay = (ms = 250) => new Promise((r) => setTimeout(r, ms));

const ME = '7Hn3mQe1dKq9vXw2YfPz8LbT5aRcUoJs4NgBxVyE6AtM';
const short = (a) => `${a.slice(0, 4)}…${a.slice(-4)}`;

const payees = [
  { name: 'Внучка Анна', address: '9xQeWvG816bUx9EPfHnxZq3JdRkaLo5SyMtBpNwC2Ue', active: true, status: 'Можно отправлять' },
  { name: 'Сосед Пётр', address: 'F3sA7kDmW2cLhVn6TqYbJ9uXoR1ePzG4iKaNvMy8CdBt', active: false, status: 'Можно будет через 31 час' },
].map((p) => ({ ...p, address_short: short(p.address) }));

const guardians = [
  { address: ME, is_me: true },
  { address: 'B6nVr2KxQm8LdYt4WcEhUs1JzPa9GfTo7iXbNe3MvRcA', is_me: false },
].map((g) => ({ ...g, address_short: short(g.address) }));

let pool = {
  address: '4kTzLp9QwXy2VbN7mRcEa1JdUhGs6oFiMt3BnYvKe8Ax',
  title: 'Семейный счёт',
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
  title: 'Личный счёт',
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

const now = () => Math.floor(Date.now() / 1000);
const history = () => [
  { signature: 's1', text: 'Пополнение на 50 USDC', time: now() - 3600 },
  { signature: 's2', text: 'Отправлено 12.5 USDC', time: now() - 86400 * 2 },
  { signature: 's3', text: 'Добавлен получатель «Внучка Анна»', time: now() - 86400 * 9 },
  { signature: 's4', text: 'Счёт открыт', time: now() - 86400 * 40 },
];

let vaults = { owner: [pool], guardian: [grandma], empty: [], waiting: [], unconfirmed: [pool], lowsol: [pool] }[scenario] ?? [pool];
let solGot = false;
let pendingTx = scenario === 'unconfirmed' ? { what: 'Отправка 5 USDC', signature: 'sig123' } : null;
let waitingPolls = 0;
let moved = 0; // every deposit/send changes the balance, as on a real chain
const setBalance = () => {
  pool.balance = `${(250.5 + moved).toFixed(2).replace(/\.?0+$/, '')} USDC`;
};

const err = (kind, message, details) => ({ kind, message, details: details ?? null });

const handlers = {
  app_info: () => ({ address: ME, address_short: short(ME), network: 'devnet' }),
  wallet: () => (!solGot && ['lowsol', 'empty', 'waiting'].includes(scenario) ? { sol: '0', low_sol: true } : { sol: '0.1', low_sol: false }),
  list_vaults: () => {
    if (scenario === 'waiting' && ++waitingPolls >= 3) vaults = [grandma];
    return vaults.map((v) => ({ address: v.address, address_short: v.address_short, mine: v.is_owner }));
  },
  load_vault: ({ vault }) => {
    const v = vaults.find((x) => x.address === vault);
    if (!v) throw err('rejected', 'Не получилось. Ничего не списано.');
    return v;
  },
  load_history: () => history(),
  pending_status: () => pendingTx,
  check_pending: () => {
    if (!pendingTx) return { status: 'none', message: 'Ничего не ожидает проверки.' };
    pendingTx = null;
    return { status: 'confirmed', message: 'Операция прошла.' };
  },
  deposit: () => {
    moved += 10;
    setBalance();
    return { signature: 'sigD', message: 'Деньги отправлены на счёт.' };
  },
  send_money: ({ amount }) => {
    if (Number(amount) > 70) {
      return { signature: 'sigR', message: 'Это больше суммы, доступной за день. Запрос отправлен: деньги уйдут, когда доверенное лицо его одобрит.' };
    }
    moved -= Number(amount);
    setBalance();
    return { signature: 'sigS', message: 'Деньги отправлены.' };
  },
  add_payee: () => ({ signature: 'sigP', message: 'Получатель добавлен. Пользоваться им можно будет через 48 часов.' }),
  add_guardian: () => ({ signature: 'sigG', message: 'Доверенное лицо добавлено.' }),
  approve_request: () => {
    grandma.pending = [];
    grandma.permissions.approve = false;
    return { signature: 'sigA', message: 'Перевод одобрен.' };
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
    if (!String(qrText).startsWith('fvpair1:')) throw err('invalid', 'Это не похоже на код с экрана устройства. Наведите камеру на QR-код ещё раз.');
    return null;
  },
  pair_with_pin: ({ pin }) => {
    if (pin === '000000') throw err('rejected', 'Устройство не приняло код. Проверьте код: на экране мог появиться новый.', 'att 0x0e');
    return null;
  },
};

export default {
  async invoke(command, args) {
    await delay();
    const handler = handlers[command];
    if (!handler) throw err('internal', `Неизвестная команда ${command}`);
    return handler(args ?? {});
  },
};
