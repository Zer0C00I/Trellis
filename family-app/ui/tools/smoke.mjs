// End-to-end smoke test of the screens against the mock backend, driving a
// real headless Chromium over the DevTools protocol (no extra packages).
//   node tools/smoke.mjs            (serves ui/ itself on a free port)
// Needs a Chromium binary: set CHROMIUM=/path if it is not "chromium".
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { readFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
import { setLocale, t } from '../i18n.js';

const root = fileURLToPath(new URL('..', import.meta.url));
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css' };

const server = createServer(async (req, res) => {
  const path = normalize(new URL(req.url, 'http://x').pathname).replace(/^(\.\.[/\\])+/, '');
  try {
    const body = await readFile(join(root, path === '/' ? 'index.html' : path));
    res.writeHead(200, { 'content-type': TYPES[extname(path)] ?? 'application/octet-stream' }).end(body);
  } catch {
    res.writeHead(404).end();
  }
});

let chrome = null;
let profile = null;
let ws = null;
let failed = 0;

try {
  await new Promise((r) => server.listen(0, '127.0.0.1', r));
  const base = `http://127.0.0.1:${server.address().port}/index.html`;

  profile = await mkdtemp(join(tmpdir(), 'smoke-'));
  chrome = spawn(process.env.CHROMIUM || 'chromium', [
    '--headless=new', '--no-sandbox', '--disable-gpu', '--remote-debugging-port=0',
    `--user-data-dir=${profile}`, '--window-size=420,900', 'about:blank',
  ], { stdio: 'ignore' });
  // Fail at once if the binary cannot be started or dies during startup.
  const startupDied = new Promise((_, reject) => {
    chrome.once('error', (e) => reject(new Error(`cannot start Chromium (${process.env.CHROMIUM || 'chromium'}): ${e.message}`)));
    chrome.once('exit', (code, signal) => reject(new Error(`Chromium exited during startup (code ${code}, signal ${signal})`)));
  });
  startupDied.catch(() => {});

  // Chromium picks a free port and writes it to DevToolsActivePort (first line).
  async function connect() {
    for (let i = 0; i < 300; i++) {
      try {
        const port = (await readFile(join(profile, 'DevToolsActivePort'), 'utf8')).split('\n')[0].trim();
        const list = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
        const page = list.find((t) => t.type === 'page');
        if (page) return page.webSocketDebuggerUrl;
      } catch { /* not up yet */ }
      await new Promise((r) => setTimeout(r, 100));
    }
    throw new Error('Chromium did not start within 30 s');
  }

  ws = new WebSocket(await Promise.race([connect(), startupDied]));
  await new Promise((r, j) => { ws.onopen = r; ws.onerror = () => j(new Error('DevTools socket failed')); });
  let id = 0;
  const pending = new Map();
  const loadWaiters = [];
  const errors = [];
  ws.onmessage = ({ data }) => {
    const m = JSON.parse(data);
    if (m.id && pending.has(m.id)) pending.get(m.id)(m);
    if (m.method === 'Page.loadEventFired') loadWaiters.splice(0).forEach((r) => r());
    if (m.method === 'Runtime.exceptionThrown') errors.push(m.params.exceptionDetails.text + ' ' + (m.params.exceptionDetails.exception?.description ?? ''));
    if (m.method === 'Runtime.consoleAPICalled' && (m.params.type === 'error' || (m.params.type === 'warning' && String(m.params.args[0]?.value).startsWith('[i18n]')))) errors.push(m.params.args.map((a) => a.value ?? a.description).join(' '));
  };
  const send = (method, params = {}) => new Promise((r) => { pending.set(++id, r); ws.send(JSON.stringify({ id, method, params })); });
  await send('Runtime.enable');
  await send('Page.enable');

  const evaluate = async (expression) => {
    const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
    if (r.result.exceptionDetails) throw new Error(r.result.exceptionDetails.exception?.description ?? 'evaluate failed');
    return r.result.result.value;
  };
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  async function until(expression, what, ms = 15000) {
    const end = Date.now() + ms;
    while (Date.now() < end) {
      if (await evaluate(expression)) return;
      await sleep(100);
    }
    throw new Error(`timed out waiting for: ${what}\npage text: ${await evaluate('document.body.innerText')}`);
  }
  const navigate = (url) => new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`no load event for ${url}`)), 15000);
    loadWaiters.push(() => { clearTimeout(timer); resolve(); });
    send('Page.navigate', { url });
  });
  // Every test starts from a blank document and a unique URL, so no DOM, timer
  // or mock state can leak from the previous test.
  let visit = 0;
  const open = async (scenario, route = 'home') => {
    await navigate('about:blank');
    await navigate(`${base}?mock=${scenario}&lang=${lang}&t=${++visit}#/${route}`);
    await until("document.getElementById('app').innerText.length > 20", `${scenario} to render`);
  };
  const text = () => evaluate('document.body.innerText');
  const click = async (label) => {
    const expr = `(() => { const e = [...document.querySelectorAll('button,a')].find(e => e.textContent.trim() === ${JSON.stringify(label)}); if (!e) return false; e.click(); return true; })()`;
    await until(expr, `a button or link "${label}" to click`);
  };
  const fill = (selector, value) => evaluate(`(() => { const e = document.querySelector(${JSON.stringify(selector)}); e.focus(); e.value = ${JSON.stringify(value)}; e.dispatchEvent(new Event('input', {bubbles:true})); })()`);
  const buttonDisabled = (label) => evaluate(`[...document.querySelectorAll('button')].find(e => e.textContent.trim() === ${JSON.stringify(label)})?.disabled`);

  let lang = 'ru';
  // The mock names its sample people in the language the app shows.
  const person = (who) => ({ en: { anna: 'Granddaughter Anna', peter: 'Neighbour Peter' }, ru: { anna: 'Внучка Анна', peter: 'Сосед Пётр' } })[lang][who];
  const tests = [];
  const test = (name, fn) => tests.push([`[${lang}] ${name}`, fn]);
  // What the page shows, from the same catalogs the page uses.
  const has = (text) => `document.body.innerText.includes(${JSON.stringify(text)})`;
  const sendOp = { key: 'op.send', params: { amount: '5 USDC' } };
  const unfinished = () => t('pending.banner', { what: sendOp });

  function define() {
  test('owner: balance, limit and role are shown', async () => {
    await open('owner');
    const page = await text();
    assert.match(page, /250\.5 USDC/);
    assert.ok(page.includes(t('home.left_today_of', { left: '70 USDC', limit: '100 USDC' })), page);
    assert.ok(page.includes(t('home.role_owner')));
  });

  test('owner: a small send goes through, a big one becomes a request', async () => {
    await open('owner', 'send');
    await until("document.querySelector('input[name=payee]')", 'payee list');
    await fill("input[type=text]", '25');
    await click(t('send.submit'));
    await until(has(t('notice.sent')), 'send ok');
    // The refresh that follows changes the balance and rebuilds the screen: the
    // success message must still be there afterwards.
    await until("import('./state.js').then(({store}) => store.vault.balance === '225.5 USDC')", 'new balance loaded');
    assert.ok((await text()).includes(t('notice.sent')));
    await fill("input[type=text]", '80');
    await click(t('send.submit'));
    await until(has(t('notice.request_filed')), 'request filed');
  });

  test('owner: a payee still in the 48h wait is not offered', async () => {
    await open('owner', 'send');
    await until("document.querySelector('input[name=payee]')", 'payee list');
    assert.equal(await evaluate("document.querySelectorAll('input[name=payee]').length"), 1);
    const page = await text();
    assert.ok(page.includes(person('peter')), page); // listed as "not yet usable", with the wait in this language
    assert.ok(page.toLocaleLowerCase(lang).includes(t('payee.wait_hours', { n: 31 }).toLocaleLowerCase(lang)), page);
  });

  test('guardian: sees the pending request and approves it', async () => {
    await open('guardian', 'approvals');
    await until(has(t('approvals.transfer', { amount: '150 USDC' })), 'request card');
    await click(t('approvals.approve'));
    await until(has(t('notice.approved')), 'approved');
    await until("!document.querySelector('.count')", 'badge gone');
  });

  test('guardian: cannot send, and is told why', async () => {
    await open('guardian', 'send');
    assert.ok((await text()).includes(t('send.why_not_owner')));
  });

  test('first launch shows the demo tag in mock mode', async () => {
    await open('empty');
    await until(has(t('pair.title')), 'first screen');
    assert.ok((await text()).includes(t('header.demo')));
  });

  test('first launch: bad text is refused, good text waits for the chain', async () => {
    await open('empty');
    await until(has(t('pair.title')), 'first screen');
    await click(t('pair.submit'));
    await until(has(t('pair.need_name')), 'name required');
    await fill('input[autocomplete=name]', 'Внук Саша');
    await fill('input[placeholder^="fvpair1"]', 'hello');
    await click(t('pair.submit'));
    await until(has(t('pair.need_code')), 'bad code refused');
    await fill('input[placeholder^="fvpair1"]', 'fvpair1:00112233445566778899aabbccddeeff');
    await click(t('pair.submit'));
    await until(has(t('waiting.title')), 'waiting');
  });

  test('first launch: waiting survives a re-render, then the vault takes over', async () => {
    await open('waiting');
    await fill('input[autocomplete=name]', 'Внук Саша');
    await fill('input[placeholder^="fvpair1"]', '123456');
    await click(t('pair.submit'));
    await until(has(t('waiting.title')), 'waiting');
    // Any re-render (here: the faucet's notice and balance refresh) must not
    // bring the pairing form back or stop the polling.
    await click(t('faucet.button'));
    await until(has(t('faucet.done')), 'faucet ok');
    await sleep(500);
    const page = await text();
    assert.ok(page.includes(t('waiting.title')));
    assert.ok(!page.includes(t('pair.title')));
    await until(has(t('home.title_personal')), 'vault shown', 30000);
  });

  test('first launch: stuck waiting can be abandoned and the code entered again', async () => {
    await open('stuck');
    await fill('input[autocomplete=name]', 'Внук Саша');
    await fill('input[placeholder^="fvpair1"]', '123456');
    await click(t('pair.submit'));
    await until(has(t('waiting.title')), 'waiting');
    await click(t('waiting.retry'));
    await until("document.querySelector('input[autocomplete=name]') && document.querySelector('input[placeholder^=\"fvpair1\"]')", 'pair form back');
    assert.ok(!(await text()).includes(t('waiting.title')));
    await fill('input[autocomplete=name]', 'Внук Саша');
    await fill('input[placeholder^="fvpair1"]', '123456');
    await click(t('pair.submit'));
    await until(has(t('waiting.title')), 'waiting again');
  });

  test('first launch: a new key can reach the address and the test-SOL button before any vault exists', async () => {
    await open('empty');
    await until(has(t('address.title')), 'address card');
    assert.ok((await text()).includes(t('address.on_address', { sol: '0' })));
    await click(t('faucet.button'));
    await until(has(t('faucet.done')), 'faucet ok');
    await until(has(t('address.on_address', { sol: '0.1' })), 'balance updated');
    assert.ok((await text()).includes(t('faucet.done'))); // message survived the re-render
  });

  test('a failed first load shows an error and a retry, not the first-launch screen', async () => {
    await open('offline');
    await until(has(t('common.retry')), 'retry button');
    const page = await text();
    assert.ok(page.includes(t('error.no_connection')));
    assert.ok(!page.includes(t('pair.title')) && !page.includes(t('open.submit')));
  });

  test('a wrong fallback code shows the plain message and keeps the details folded', async () => {
    await open('empty');
    await fill('input[autocomplete=name]', 'Внук Саша');
    await fill('input[placeholder^="fvpair1"]', '000000');
    await click(t('pair.submit'));
    await until(has(t('error.pairing.write_rejected')), 'rejection message');
    assert.equal(await evaluate("document.querySelector('details')?.open"), false);
  });

  test('an unconfirmed operation shows the warning, and its result stays visible after the check', async () => {
    await open('unconfirmed');
    await until(has(unfinished()), 'warning');
    await click(t('pending.check'));
    await until(`!${has(unfinished())}`, 'warning cleared');
    await sleep(300);
    assert.ok((await text()).includes(t('resolution.confirmed')));
  });

  test('a failed check is shown as an info notice, a confirmed one as a success notice', async () => {
    await open('unconfirmed-failed');
    await until(has(unfinished()), 'warning');
    await click(t('pending.check'));
    await until("document.getElementById('notice')", 'notice');
    assert.equal(await evaluate("document.getElementById('notice').className"), 'banner banner-info');
    assert.ok((await text()).includes(t('resolution.failed')));
    await open('unconfirmed');
    await until(has(unfinished()), 'warning');
    await click(t('pending.check'));
    await until("document.getElementById('notice')", 'notice');
    assert.equal(await evaluate("document.getElementById('notice').className"), 'banner banner-ok');
  });

  test('while an operation is unconfirmed the money buttons are disabled', async () => {
    await open('unconfirmed', 'send');
    await until("document.querySelector('input[name=payee]')", 'payee list');
    assert.equal(await buttonDisabled(t('send.submit')), true);
    assert.ok((await text()).includes(t('pending.lock_hint')));
    await click(t('pending.check'));
    await until(`!${has(unfinished())}`, 'warning cleared');
    await until("document.querySelector('input[name=payee]')", 'send form back');
    assert.equal(await buttonDisabled(t('send.submit')), false);
  });

  test('a background refresh is deferred while the person types, then applied', async () => {
    await open('owner');
    assert.match(await text(), /250\.5 USDC/);
    await click(t('home.deposit'));
    await until("document.querySelector('input[type=text]')", 'amount field');
    await fill('input[type=text]', '12.5');
    await evaluate('globalThis.__mockBump()');
    await evaluate("document.dispatchEvent(new Event('visibilitychange'))");
    await sleep(1500);
    assert.equal(await evaluate("document.querySelector('input[type=text]').value"), '12.5');
    assert.match(await text(), /250\.5 USDC/); // the update waits
    await evaluate('document.activeElement.blur()');
    await evaluate("document.dispatchEvent(new Event('visibilitychange'))");
    await until(has('251.5 USDC'), 'deferred balance applied');
  });

  test('an unknown notice kind falls back to the info banner', async () => {
    await open('owner');
    await evaluate("__ctx.notify({ key: 'notice.sent' }, 'bogus'); __ctx.notify({ key: 'notice.deposit' }, null);");
    assert.equal(await evaluate("document.getElementById('notice').className"), 'banner banner-info');
    await evaluate("__ctx.notify({ key: 'notice.sent' })");
    assert.equal(await evaluate("document.getElementById('notice').className"), 'banner banner-ok');
  });

  test('a check that is still undecided shows a warn notice and keeps the warning', async () => {
    await open('unconfirmed-waiting');
    await until(has(unfinished()), 'warning');
    await click(t('pending.check'));
    await until("document.getElementById('notice')", 'notice');
    assert.equal(await evaluate("document.getElementById('notice').className"), 'banner banner-warn');
    await sleep(500);
    assert.ok((await text()).includes(unfinished()));
  });

  test('notify keeps half-typed fields, renders no second notice and is announced', async () => {
    await open('owner', 'people');
    await until("document.querySelector('input[type=text]')", 'name field');
    await fill('input[type=text]', 'Баба Нюра');
    await evaluate("__ctx.notify({ key: 'notice.sent' })");
    assert.equal(await evaluate("document.querySelector('input[type=text]').value"), 'Баба Нюра');
    assert.equal(await evaluate("document.querySelectorAll('#notice').length"), 1);
    assert.ok((await text()).includes(t('notice.sent')));
    await evaluate("__ctx.notify({ key: 'notice.deposit' })");
    assert.equal(await evaluate("document.querySelectorAll('#notice').length"), 1);
    assert.equal(await evaluate("document.querySelector('input[type=text]').value"), 'Баба Нюра');
    await until(`document.getElementById('live').textContent === ${JSON.stringify(t('notice.deposit'))}`, 'live region text');
  });

  test('a slow older response never overwrites a newer one', async () => {
    await open('owner');
    assert.match(await text(), /250\.5 USDC/);
    // Slow call: its load_vault answer is computed now-ish (251.5) but held back
    // 1500 ms. The mock deletes the hook entry the moment it has computed it.
    await evaluate("globalThis.__mockDelay = { load_vault: 1500 }; __mockBump(); void __ctx.refresh({ background: true });");
    await until('!globalThis.__mockDelay.load_vault', 'the slow call to have read its (older) balance');
    // Fast call: reads the newer 252.5 and lands long before the slow one.
    await evaluate('__mockBump(); void __ctx.refresh();');
    await until(has('252.5 USDC'), 'newer balance');
    await sleep(2200); // the slow call (1500 ms hold + history) has landed by now
    assert.match(await text(), /252\.5 USDC/);
    assert.doesNotMatch(await text(), /251\.5 USDC/);
  });

  test('a newer call skipped because of typing does not lose an older result', async () => {
    await open('owner');
    await click(t('home.deposit'));
    await until("document.querySelector('input[type=text]')", 'amount field');
    await fill('input[type=text]', '5');
    await evaluate("globalThis.__mockDelay = { load_vault: 800 }; __mockBump(); void __ctx.refresh({ background: true }); void __ctx.refresh({ background: true });");
    await sleep(1000); // the fast call finished while typing and was skipped
    assert.match(await text(), /250\.5 USDC/);
    await evaluate('document.activeElement.blur()');
    await until(has('251.5 USDC'), 'older result still applied');
  });

  test('an unconfirmed error leaves the button disabled the whole time', async () => {
    await open('send-unconfirmed', 'send');
    await until("document.querySelector('input[name=payee]')", 'payee list');
    await fill('input[type=text]', '5');
    await evaluate(`(() => {
      const b = [...document.querySelectorAll('button')].find((e) => e.textContent.trim() === ${JSON.stringify(t('send.submit'))});
      window.__log = [];
      new MutationObserver((rs) => rs.forEach((r) => window.__log.push(r.oldValue === null ? 'disabled' : 'enabled')))
        .observe(b, { attributes: true, attributeFilter: ['disabled'], attributeOldValue: true });
    })()`);
    await click(t('send.submit'));
    await until(has(unfinished()), 'warning');
    await sleep(300);
    assert.equal(await buttonDisabled(t('send.submit')), true);
    assert.deepEqual(await evaluate('window.__log'), ['disabled']);
  });

  test('the language can be switched on the Me tab, is remembered and rewords the screen', async () => {
    const other = lang === 'ru' ? 'en' : 'ru';
    const names = { en: 'English', ru: 'Русский' };
    try {
      await open('owner', 'me');
      assert.ok((await text()).includes(t('me.address_title')));
      assert.equal(await evaluate('document.documentElement.lang'), lang);
      await click(names[other]);
      setLocale(other, { persist: false });
      await until(has(t('me.address_title')), 'the screen in the other language');
      assert.equal(await evaluate('document.documentElement.lang'), other);
      assert.equal(await evaluate("document.querySelector('[aria-pressed=true]').textContent.trim()"), names[other]);
      // Reload without "?lang": the saved choice wins over the system language.
      await navigate('about:blank');
      await navigate(`${base}?mock=owner&t=${++visit}#/home`);
      await until("document.getElementById('app').innerText.length > 20", 'reload');
      assert.ok((await text()).includes(t('home.role_owner')));
    } finally {
      setLocale(lang, { persist: false });
      await evaluate('localStorage.clear()');
    }
  });

  test('a notice already on screen is reworded when the language changes', async () => {
    const other = lang === 'ru' ? 'en' : 'ru';
    const names = { en: 'English', ru: 'Русский' };
    try {
      await open('owner', 'me');
      await evaluate("__ctx.notify({ key: 'notice.sent' })");
      await until(has(t('notice.sent')), 'notice');
      await click(names[other]);
      setLocale(other, { persist: false });
      await until(has(t('notice.sent')), 'the notice in the other language');
    } finally {
      setLocale(lang, { persist: false });
      await evaluate('localStorage.clear()');
    }
  });

  test('no screen is wider than a 360px phone, with the longest text of each language', async () => {
    await send('Emulation.setDeviceMetricsOverride', { width: 360, height: 800, deviceScaleFactor: 1, mobile: false });
    const wide = `(() => {
      const width = document.documentElement.clientWidth;
      const bad = [...document.querySelectorAll('#app *')]
        .filter((e) => e.getBoundingClientRect().right > width + 0.5 && getComputedStyle(e).position !== 'fixed')
        .map((e) => e.tagName.toLowerCase() + '.' + e.className + ': ' + (e.textContent || '').trim().slice(0, 40));
      return { scroll: document.documentElement.scrollWidth, width, bad: bad.slice(0, 5) };
    })()`;
    try {
      for (const [scenario, route] of [
        ['owner', 'home'], ['owner', 'send'], ['owner', 'people'], ['owner', 'me'],
        ['guardian', 'approvals'], ['guardian', 'home'], ['empty', 'home'], ['unconfirmed', 'home'],
      ]) {
        await open(scenario, route);
        await sleep(400);
        const r = await evaluate(wide);
        assert.ok(r.scroll <= r.width && r.bad.length === 0, `${scenario}/${route}: ${JSON.stringify(r)}`);
      }
      await open('waiting');
      await fill('input[autocomplete=name]', 'Внук Саша');
      await fill('input[placeholder^="fvpair1"]', '123456');
      await click(t('pair.submit'));
      await until(has(t('waiting.title')), 'waiting');
      const r = await evaluate(wide);
      assert.ok(r.scroll <= r.width && r.bad.length === 0, `waiting: ${JSON.stringify(r)}`);
    } finally {
      await send('Emulation.clearDeviceMetricsOverride');
    }
  });

  test('names from the chain are shown as text, never as HTML', async () => {
    await open('owner', 'people');
    await until(has(person('anna')), 'people list');
    await evaluate(`import('./dom.js').then(({h}) => { document.body.append(h('div', {id:'x'}, '<img src=x onerror=window.__pwned=1>')); })`);
    await sleep(200);
    assert.equal(await evaluate('window.__pwned === undefined && !document.querySelector("#x img")'), true);
  });
  }

  for (const language of ['ru', 'en']) {
    lang = language;
    setLocale(language, { persist: false });
    tests.length = 0;
    define();
    for (const [name, fn] of tests) {
      try {
        await fn();
        console.log(`ok   ${name}`);
      } catch (e) {
        failed++;
        console.log(`FAIL ${name}\n     ${String(e.message).split('\n').join('\n     ')}`);
      }
    }
  }
  if (errors.length) {
    failed++;
    console.log(`FAIL console errors:\n     ${errors.join('\n     ')}`);
  }
} catch (e) {
  failed++;
  console.log(`FAIL harness: ${e.stack ?? e}`);
} finally {
  try { ws?.close(); } catch { /* already closed */ }
  if (chrome?.pid && chrome.exitCode === null && chrome.signalCode === null) {
    const exited = new Promise((r) => chrome.once('exit', r));
    chrome.kill();
    await Promise.race([exited, new Promise((r) => setTimeout(r, 5000))]);
    if (chrome.exitCode === null) chrome.kill('SIGKILL');
  }
  server.closeAllConnections?.();
  server.close();
  if (profile) await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }).catch(() => {});
}
console.log(failed ? `${failed} failed` : 'all passed');
process.exit(failed ? 1 : 0);
