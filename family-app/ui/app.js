// The shell: loads state, shows a screen, keeps it fresh.
import { call, isMock } from './api.js';
import { initLocale, setLocale, t, tMsg } from './i18n.js';
import { waitingDelayMs } from './poll.js';
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
  ['home', 'tabs.home'],
  ['send', 'tabs.send'],
  ['people', 'tabs.people'],
  ['approvals', 'tabs.approvals'],
  ['me', 'tabs.me'],
];
const REFRESH_MS = 60_000;
const NOTICE_MS = 10_000;
const KINDS = ['ok', 'info', 'warn'];
const SELECTED_KEY = 'trellis.selectedVault';

// The language: the saved choice, else the system's. A demo page may force one
// with "?lang=ru" (never inside the real app, like "?mock").
initLocale();
if (isMock) {
  const forced = new URLSearchParams(globalThis.location?.search ?? '').get('lang');
  if (forced) setLocale(forced, { persist: false });
}

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
  // Words everything again in the chosen language; nobody is typing in the
  // button that triggered this, and a notice on screen is re-worded too.
  setLanguage: (language) => {
    setLocale(language);
    render();
  },
  setWaiting: (waiting) => update({ waiting, pairHint: !waiting && store.waiting }),
  // A success message ({ key, params }, as Rust sends it) that outlives the
  // re-render that fresh data causes; it is worded again if the language changes.
  notify: (message, kind = 'ok') => {
    store.notice = message;
    store.noticeKind = KINDS.includes(kind) ? kind : 'info';
    clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => {
      store.notice = null;
      document.getElementById('notice')?.remove(); // no re-render: nobody's typing is interrupted
      const live = document.getElementById('live');
      if (live) live.textContent = ''; // no stale announcement
    }, NOTICE_MS);
    // Never re-render the screen for this: half-typed fields must survive.
    const fresh = noticeBanner();
    const existing = document.getElementById('notice');
    const main = root.querySelector('main.content');
    if (existing) existing.replaceWith(fresh);
    else if (main) main.prepend(fresh);
    else render();
    announce(tMsg(message));
  },
};
if (isMock) globalThis.__ctx = ctx; // test hook, never in the real app

// One live region outside #app: re-renders cannot swallow the announcement.
function announce(text) {
  const live = document.getElementById('live');
  if (!live) return;
  live.textContent = '';
  setTimeout(() => {
    live.textContent = text;
  }, 50);
}

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
let appliedSeq = 0; // newest call whose result is on screen (or was identical to it)

// `background` refreshes (timers, returning to the app) change nothing on
// screen unless the data really changed, and never interrupt typing.
// `select` is the vault the person asked for: it only becomes the selection
// once it has loaded. A result is used only if it is newer than the last one
// applied, so a slow older answer never overwrites a fresher one; a call that
// was skipped just because the person is typing does not count as applied, so
// an older result can still land afterwards.
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
    if (seq < appliedSeq) return;
    if (selected) remember(selected);
    const snapshot = JSON.stringify({ wallet, vaults, selected, vault, history });
    const unchanged = store.loaded && !store.loadError && snapshot === lastSnapshot;
    if (unchanged) {
      appliedSeq = seq;
      return;
    }
    if (background && isTyping()) return;
    appliedSeq = seq;
    lastSnapshot = snapshot;
    update({ wallet, vaults, selected, vault, history, loaded: true, everLoaded: true, loadError: null, waiting: vault ? false : store.waiting });
  } catch (e) {
    if (seq < appliedSeq || (background && isTyping())) return;
    appliedSeq = seq;
    update({ loaded: true, loadError: e });
  }
}

function header() {
  const picker =
    store.vaults.length > 1
      ? h(
          'select',
          {
            'aria-label': t('header.pick_vault'),
            onchange: (ev) => loadAll({ select: ev.target.value }),
          },
          store.vaults.map((v) => h('option', { value: v.address, selected: v.address === store.selected }, t(v.mine ? 'vault.mine' : 'vault.other', { address: v.address_short }))),
        )
      : null;
  return h('header', { class: 'top' }, h('span', { class: 'logo', role: 'img', 'aria-label': 'Trellis' }), isMock ? h('span', { class: 'tag' }, t('header.demo')) : null, picker);
}

function pendingBanner() {
  if (!store.pending) return null;
  const out = h('div', { 'aria-live': 'polite' });
  const check = button(t('pending.check'), async () => {
    check.disabled = true;
    try {
      const r = await call('check_pending');
      // survives the re-render below; a check that did not go through is not a success
      ctx.notify(r, { confirmed: 'ok', waiting: 'warn' }[r.status] ?? 'info');
      await loadPending();
      if (r.status !== 'waiting') await loadAll();
    } catch (e) {
      out.replaceChildren(errorBanner(e));
    } finally {
      check.disabled = false;
    }
  });
  return banner('warn', t('pending.banner', { what: store.pending.what }), check, out);
}

function noticeBanner() {
  if (!store.notice) return null;
  const el = banner(store.noticeKind, tMsg(store.notice));
  el.id = 'notice';
  return el;
}

function nav(current) {
  const waiting = store.vault?.permissions.approve ? store.vault.pending.length : 0;
  return h(
    'nav',
    { class: 'tabs', 'aria-label': t('nav.label') },
    TABS.map(([name, labelKey]) =>
      h('a', { href: `#/${name}`, class: name === current ? 'tab current' : 'tab', 'aria-current': name === current ? 'page' : null }, t(labelKey), name === 'approvals' && waiting ? h('span', { class: 'count' }, waiting) : null),
    ),
  );
}

function render() {
  for (const fn of leaveHooks) fn();
  leaveHooks = [];
  clear(root);
  if (!store.loaded) return root.append(h('p', { class: 'center' }, t('common.loading')));

  if (!store.vault && store.loadError && !store.everLoaded) {
    // Never offer "open a new account" because of a network error.
    root.append(h('main', { class: 'content' }, h('h1', {}, t('common.load_failed')), errorBanner(store.loadError), button(t('common.retry'), retry)));
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
// First launch, code accepted: look for the new vault, backing off as the wait
// drags on. Nothing is scheduled unless we are waiting without a vault.
let waitTimer = null;
let waitAttempt = 0;
let wasWaiting = false;
function syncWaitingPoll() {
  if (!(store.waiting && !store.vault)) {
    clearTimeout(waitTimer);
    waitTimer = null;
    wasWaiting = false;
    return;
  }
  if (!wasWaiting) {
    wasWaiting = true;
    waitAttempt = 0;
  }
  if (waitTimer !== null) return;
  waitTimer = setTimeout(async () => {
    waitTimer = null;
    waitAttempt++;
    await loadAll({ background: true });
    syncWaitingPoll();
  }, waitingDelayMs(waitAttempt));
}
subscribe(syncWaitingPoll);

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
