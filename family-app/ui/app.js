// The shell: loads state, shows a screen, keeps it fresh.
import { call, isMock } from './api.js';
import { banner, button, errorBanner } from './components.js';
import { clear, h } from './dom.js';
import { parseRoute } from './router.js';
import { store, subscribe, update } from './state.js';
import * as approvals from './screens/approvals.js';
import * as first from './screens/first.js';
import * as home from './screens/home.js';
import * as me from './screens/me.js';
import * as people from './screens/people.js';
import * as send from './screens/send.js';

const SCREENS = { home, send, people, approvals, me };
const TABS = [
  ['home', 'Счёт'],
  ['send', 'Отправить'],
  ['people', 'Люди'],
  ['approvals', 'Одобрения'],
  ['me', 'Я'],
];
const REFRESH_MS = 60_000;
const WAITING_POLL_MS = 5_000;
const NOTICE_MS = 10_000;
const SELECTED_KEY = 'trellis.selectedVault';

const root = document.getElementById('app');
let leaveHooks = [];
let noticeTimer;

const ctx = {
  store,
  refresh: (options) => loadAll(options),
  go: (route) => {
    location.hash = `#/${route}`;
  },
  onLeave: (fn) => leaveHooks.push(fn),
  setWaiting: (waiting) => update({ waiting, pairHint: !waiting && store.waiting }),
  // A success message that outlives the re-render that fresh data causes.
  notify: (text, kind = 'ok') => {
    store.notice = text;
    store.noticeKind = kind;
    clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => {
      store.notice = null;
      document.getElementById('notice')?.remove(); // no re-render: nobody's typing is interrupted
    }, NOTICE_MS);
    document.getElementById('notice')?.replaceWith(noticeBanner() ?? document.createComment(''));
    if (!document.getElementById('notice')) render();
  },
};

function remember(address) {
  try {
    localStorage.setItem(SELECTED_KEY, address);
  } catch {
    /* storage may be blocked; the first vault is a fine default */
  }
}

function remembered() {
  try {
    return localStorage.getItem(SELECTED_KEY);
  } catch {
    return null;
  }
}

async function loadPending() {
  try {
    update({ pending: await call('pending_status') });
  } catch (e) {
    update({ loadError: e });
  }
}

// True while the person is typing in a field: a background refresh must not
// rebuild the screen under their fingers.
function isTyping() {
  const el = document.activeElement;
  return Boolean(el && root.contains(el) && ['INPUT', 'SELECT', 'TEXTAREA'].includes(el.tagName));
}

let lastSnapshot = '';
let loadSeq = 0;

// `background` refreshes (timers, returning to the app) change nothing on
// screen unless the data really changed, and never interrupt typing.
// `select` is the vault the person asked for: it only becomes the selection
// once it has loaded. Only the newest call may write the store, so a slow
// older answer never overwrites a fresher one.
async function loadAll({ background = false, select = null } = {}) {
  const seq = ++loadSeq;
  try {
    const [wallet, vaults] = await Promise.all([call('wallet'), call('list_vaults')]);
    let selected = select ?? store.selected;
    if (!vaults.some((v) => v.address === selected)) {
      selected = vaults.find((v) => v.address === remembered())?.address ?? vaults[0]?.address ?? null;
    }
    let vault = null;
    let history = [];
    if (selected) {
      vault = await call('load_vault', { vault: selected });
      history = await call('load_history', { vault: selected, decimals: vault.decimals });
    }
    if (seq !== loadSeq) return;
    if (selected) remember(selected);
    const snapshot = JSON.stringify({ wallet, vaults, selected, vault, history });
    const unchanged = store.loaded && !store.loadError && snapshot === lastSnapshot;
    if (unchanged || (background && isTyping())) return;
    lastSnapshot = snapshot;
    update({ wallet, vaults, selected, vault, history, loaded: true, everLoaded: true, loadError: null, waiting: vault ? false : store.waiting });
  } catch (e) {
    if (seq !== loadSeq || (background && isTyping())) return;
    update({ loaded: true, loadError: e });
  }
}

function header() {
  const picker =
    store.vaults.length > 1
      ? h(
          'select',
          {
            'aria-label': 'Выбрать счёт',
            onchange: (ev) => loadAll({ select: ev.target.value }),
          },
          store.vaults.map((v) => h('option', { value: v.address, selected: v.address === store.selected }, `${v.mine ? 'Мой счёт' : 'Счёт родственника'} ${v.address_short}`)),
        )
      : null;
  return h('header', { class: 'top' }, h('strong', {}, 'Trellis'), isMock ? h('span', { class: 'tag' }, 'демо') : null, picker);
}

function pendingBanner() {
  if (!store.pending) return null;
  const out = h('div', { 'aria-live': 'polite' });
  const check = button('Проверить', async () => {
    check.disabled = true;
    try {
      const r = await call('check_pending');
      // survives the re-render below; a check that did not go through is not a success
      ctx.notify(r.message, { confirmed: 'ok', waiting: 'warn' }[r.status] ?? 'info');
      await loadPending();
      if (r.status !== 'waiting') await loadAll();
    } catch (e) {
      out.replaceChildren(errorBanner(e));
    } finally {
      check.disabled = false;
    }
  });
  return banner('warn', `Не закончена операция: «${store.pending.what}». Пока мы не узнаем, чем она кончилась, другие переводы недоступны, чтобы не заплатить дважды.`, check, out);
}

function noticeBanner() {
  if (!store.notice) return null;
  const el = banner(store.noticeKind, store.notice);
  el.id = 'notice';
  return el;
}

function nav(current) {
  const waiting = store.vault?.permissions.approve ? store.vault.pending.length : 0;
  return h(
    'nav',
    { class: 'tabs', 'aria-label': 'Разделы' },
    TABS.map(([name, label]) =>
      h('a', { href: `#/${name}`, class: name === current ? 'tab current' : 'tab', 'aria-current': name === current ? 'page' : null }, label, name === 'approvals' && waiting ? h('span', { class: 'count' }, waiting) : null),
    ),
  );
}

function render() {
  for (const fn of leaveHooks) fn();
  leaveHooks = [];
  clear(root);
  if (!store.loaded) return root.append(h('p', { class: 'center' }, 'Загрузка…'));

  if (!store.vault && store.loadError && !store.everLoaded) {
    // Never offer "open a new account" because of a network error.
    root.append(h('main', { class: 'content' }, h('h1', {}, 'Не удалось загрузить'), errorBanner(store.loadError), button('Повторить', retry)));
    return;
  }
  if (!store.vault) {
    root.append(h('main', { class: 'content' }, noticeBanner(), store.loadError ? errorBanner(store.loadError) : null, pendingBanner(), first.render(ctx)));
    return;
  }
  const route = parseRoute(location.hash);
  root.append(
    header(),
    h('main', { class: 'content' }, noticeBanner(), store.loadError ? errorBanner(store.loadError) : null, pendingBanner(), SCREENS[route].render(ctx)),
    nav(route),
  );
}

subscribe(render);
window.addEventListener('hashchange', render);
document.addEventListener('trellis:pending', loadPending);
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'visible') loadAll({ background: true });
});
setInterval(() => {
  if (document.visibilityState === 'visible') loadAll({ background: true });
}, REFRESH_MS);
// First launch, code accepted: look for the new vault every few seconds. The
// tick does nothing once a vault is on screen (loadAll clears `waiting`).
setInterval(() => {
  if (store.waiting && !store.vault) loadAll({ background: true });
}, WAITING_POLL_MS);

function retry() {
  update({ loaded: false, loadError: null });
  start();
}

async function start() {
  render();
  try {
    update({ info: await call('app_info') });
  } catch (e) {
    update({ loadError: e });
  }
  await loadPending();
  await loadAll();
}

start();
