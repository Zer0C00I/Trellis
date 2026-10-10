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
    if (m.method === 'Runtime.consoleAPICalled' && m.params.type === 'error') errors.push(m.params.args.map((a) => a.value ?? a.description).join(' '));
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
    await navigate(`${base}?mock=${scenario}&t=${++visit}#/${route}`);
    await until("document.getElementById('app').innerText.length > 20", `${scenario} to render`);
  };
  const text = () => evaluate('document.body.innerText');
  const click = async (label) => {
    const expr = `(() => { const e = [...document.querySelectorAll('button,a')].find(e => e.textContent.trim() === ${JSON.stringify(label)}); if (!e) return false; e.click(); return true; })()`;
    await until(expr, `a button or link "${label}" to click`);
  };
  const fill = (selector, value) => evaluate(`(() => { const e = document.querySelector(${JSON.stringify(selector)}); e.focus(); e.value = ${JSON.stringify(value)}; e.dispatchEvent(new Event('input', {bubbles:true})); })()`);
  const buttonDisabled = (label) => evaluate(`[...document.querySelectorAll('button')].find(e => e.textContent.trim() === ${JSON.stringify(label)})?.disabled`);

  const tests = [];
  const test = (name, fn) => tests.push([name, fn]);

  test('owner: balance, limit and role are shown', async () => {
    await open('owner');
    const t = await text();
    assert.match(t, /250\.5 USDC/);
    assert.match(t, /ещё 70 USDC из 100 USDC/);
    assert.match(t, /Вы владелец счёта/);
  });

  test('owner: a small send goes through, a big one becomes a request', async () => {
    await open('owner', 'send');
    await until("document.querySelector('input[name=payee]')", 'payee list');
    await fill("input[type=text]", '25');
    await click('Отправить');
    await until("document.body.innerText.includes('Деньги отправлены.')", 'send ok');
    // The refresh that follows changes the balance and rebuilds the screen: the
    // success message must still be there afterwards.
    await until("import('./state.js').then(({store}) => store.vault.balance === '225.5 USDC')", 'new balance loaded');
    assert.match(await text(), /Деньги отправлены\./);
    await fill("input[type=text]", '80');
    await click('Отправить');
    await until("document.body.innerText.includes('Запрос отправлен')", 'request filed');
  });

  test('owner: a payee still in the 48h wait is not offered', async () => {
    await open('owner', 'send');
    await until("document.querySelector('input[name=payee]')", 'payee list');
    assert.equal(await evaluate("document.querySelectorAll('input[name=payee]').length"), 1);
    assert.match(await text(), /Сосед Пётр/); // listed as "not yet usable"
  });

  test('guardian: sees the pending request and approves it', async () => {
    await open('guardian', 'approvals');
    await until("document.body.innerText.includes('Перевод 150 USDC')", 'request card');
    await click('Одобрить');
    await until("document.body.innerText.includes('Перевод одобрен.')", 'approved');
    await until("!document.querySelector('.count')", 'badge gone');
  });

  test('guardian: cannot send, and is told why', async () => {
    await open('guardian', 'send');
    assert.match(await text(), /только его владелец/);
  });

  test('first launch shows the demo tag in mock mode', async () => {
    await open('empty');
    await until("document.body.innerText.includes('Меня добавляет родственник')", 'first screen');
    assert.match(await text(), /демо/);
  });

  test('first launch: bad text is refused, good text waits for the chain', async () => {
    await open('empty');
    await until("document.body.innerText.includes('Меня добавляет родственник')", 'first screen');
    await click('Передать устройству');
    await until("document.body.innerText.includes('Напишите своё имя')", 'name required');
    await fill('input[autocomplete=name]', 'Внук Саша');
    await fill('input[placeholder^="fvpair1"]', 'hello');
    await click('Передать устройству');
    await until("document.body.innerText.includes('Введите текст QR-кода')", 'bad code refused');
    await fill('input[placeholder^="fvpair1"]', 'fvpair1:00112233445566778899aabbccddeeff');
    await click('Передать устройству');
    await until("document.body.innerText.includes('Ждём, пока вас добавят')", 'waiting');
  });

  test('first launch: waiting survives a re-render, then the vault takes over', async () => {
    await open('waiting');
    await fill('input[autocomplete=name]', 'Внук Саша');
    await fill('input[placeholder^="fvpair1"]', '123456');
    await click('Передать устройству');
    await until("document.body.innerText.includes('Ждём, пока вас добавят')", 'waiting');
    // Any re-render (here: the faucet's notice and balance refresh) must not
    // bring the pairing form back or stop the polling.
    await click('Получить тестовые SOL');
    await until("document.body.innerText.includes('SOL пришли на ваш адрес')", 'faucet ok');
    await sleep(500);
    let t = await text();
    assert.match(t, /Ждём, пока вас добавят/);
    assert.doesNotMatch(t, /Меня добавляет родственник/);
    await until("document.body.innerText.includes('Личный счёт')", 'vault shown', 30000);
  });

  test('first launch: stuck waiting can be abandoned and the code entered again', async () => {
    await open('stuck');
    await fill('input[autocomplete=name]', 'Внук Саша');
    await fill('input[placeholder^="fvpair1"]', '123456');
    await click('Передать устройству');
    await until("document.body.innerText.includes('Ждём, пока вас добавят')", 'waiting');
    await click('Ввести код заново');
    await until("document.querySelector('input[autocomplete=name]') && document.querySelector('input[placeholder^=\"fvpair1\"]')", 'pair form back');
    assert.doesNotMatch(await text(), /Ждём, пока вас добавят/);
    await fill('input[autocomplete=name]', 'Внук Саша');
    await fill('input[placeholder^="fvpair1"]', '123456');
    await click('Передать устройству');
    await until("document.body.innerText.includes('Ждём, пока вас добавят')", 'waiting again');
  });

  test('first launch: a new key can reach the address and the test-SOL button before any vault exists', async () => {
    await open('empty');
    await until("document.body.innerText.includes('Ваш адрес и комиссии')", 'address card');
    assert.match(await text(), /На адресе: 0 SOL/);
    await click('Получить тестовые SOL');
    await until("document.body.innerText.includes('SOL пришли на ваш адрес')", 'faucet ok');
    await until("document.body.innerText.includes('На адресе: 0.1 SOL')", 'balance updated');
    assert.match(await text(), /SOL пришли на ваш адрес/); // message survived the re-render
  });

  test('a failed first load shows an error and a retry, not the first-launch screen', async () => {
    await open('offline');
    await until("document.body.innerText.includes('Повторить')", 'retry button');
    const t = await text();
    assert.match(t, /Нет связи/);
    assert.doesNotMatch(t, /Меня добавляет родственник|Открыть семейный счёт/);
  });

  test('a wrong fallback code shows the plain message and keeps the details folded', async () => {
    await open('empty');
    await fill('input[autocomplete=name]', 'Внук Саша');
    await fill('input[placeholder^="fvpair1"]', '000000');
    await click('Передать устройству');
    await until("document.body.innerText.includes('Устройство не приняло код')", 'rejection message');
    assert.equal(await evaluate("document.querySelector('details')?.open"), false);
  });

  test('an unconfirmed operation shows the warning, and its result stays visible after the check', async () => {
    await open('unconfirmed');
    await until("document.body.innerText.includes('Не закончена операция')", 'warning');
    await click('Проверить');
    await until("!document.body.innerText.includes('Не закончена операция')", 'warning cleared');
    await sleep(300);
    assert.match(await text(), /Операция прошла\./);
  });

  test('a failed check is shown as an info notice, a confirmed one as a success notice', async () => {
    await open('unconfirmed-failed');
    await until("document.body.innerText.includes('Не закончена операция')", 'warning');
    await click('Проверить');
    await until("document.getElementById('notice')", 'notice');
    assert.equal(await evaluate("document.getElementById('notice').className"), 'banner banner-info');
    assert.match(await text(), /деньги не списаны/);
    await open('unconfirmed');
    await until("document.body.innerText.includes('Не закончена операция')", 'warning');
    await click('Проверить');
    await until("document.getElementById('notice')", 'notice');
    assert.equal(await evaluate("document.getElementById('notice').className"), 'banner banner-ok');
  });

  test('while an operation is unconfirmed the money buttons are disabled', async () => {
    await open('unconfirmed', 'send');
    await until("document.querySelector('input[name=payee]')", 'payee list');
    assert.equal(await buttonDisabled('Отправить'), true);
    assert.match(await text(), /Сначала проверьте прошлую операцию/);
    await click('Проверить');
    await until("!document.body.innerText.includes('Не закончена операция')", 'warning cleared');
    await until("document.querySelector('input[name=payee]')", 'send form back');
    assert.equal(await buttonDisabled('Отправить'), false);
  });

  test('a background refresh is deferred while the person types, then applied', async () => {
    await open('owner');
    assert.match(await text(), /250\.5 USDC/);
    await click('Пополнить');
    await until("document.querySelector('input[type=text]')", 'amount field');
    await fill('input[type=text]', '12.5');
    await evaluate('globalThis.__mockBump()');
    await evaluate("document.dispatchEvent(new Event('visibilitychange'))");
    await sleep(1500);
    assert.equal(await evaluate("document.querySelector('input[type=text]').value"), '12.5');
    assert.match(await text(), /250\.5 USDC/); // the update waits
    await evaluate('document.activeElement.blur()');
    await evaluate("document.dispatchEvent(new Event('visibilitychange'))");
    await until("document.body.innerText.includes('251.5 USDC')", 'deferred balance applied');
  });

  test('an unknown notice kind falls back to the info banner', async () => {
    await open('owner');
    await evaluate("__ctx.notify('Что-то', 'bogus'); __ctx.notify('Ещё', null);");
    assert.equal(await evaluate("document.getElementById('notice').className"), 'banner banner-info');
    await evaluate("__ctx.notify('Хорошо')");
    assert.equal(await evaluate("document.getElementById('notice').className"), 'banner banner-ok');
  });

  test('a check that is still undecided shows a warn notice and keeps the warning', async () => {
    await open('unconfirmed-waiting');
    await until("document.body.innerText.includes('Не закончена операция')", 'warning');
    await click('Проверить');
    await until("document.getElementById('notice')", 'notice');
    assert.equal(await evaluate("document.getElementById('notice').className"), 'banner banner-warn');
    await sleep(500);
    assert.match(await text(), /Не закончена операция/);
  });

  test('notify keeps half-typed fields, renders no second notice and is announced', async () => {
    await open('owner', 'people');
    await until("document.querySelector('input[type=text]')", 'name field');
    await fill('input[type=text]', 'Баба Нюра');
    await evaluate("__ctx.notify('Готово')");
    assert.equal(await evaluate("document.querySelector('input[type=text]').value"), 'Баба Нюра');
    assert.equal(await evaluate("document.querySelectorAll('#notice').length"), 1);
    assert.match(await text(), /Готово/);
    await evaluate("__ctx.notify('Готово 2')");
    assert.equal(await evaluate("document.querySelectorAll('#notice').length"), 1);
    assert.equal(await evaluate("document.querySelector('input[type=text]').value"), 'Баба Нюра');
    await until("document.getElementById('live').textContent === 'Готово 2'", 'live region text');
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
    await until("document.body.innerText.includes('252.5 USDC')", 'newer balance');
    await sleep(2200); // the slow call (1500 ms hold + history) has landed by now
    assert.match(await text(), /252\.5 USDC/);
    assert.doesNotMatch(await text(), /251\.5 USDC/);
  });

  test('a newer call skipped because of typing does not lose an older result', async () => {
    await open('owner');
    await click('Пополнить');
    await until("document.querySelector('input[type=text]')", 'amount field');
    await fill('input[type=text]', '5');
    await evaluate("globalThis.__mockDelay = { load_vault: 800 }; __mockBump(); void __ctx.refresh({ background: true }); void __ctx.refresh({ background: true });");
    await sleep(1000); // the fast call finished while typing and was skipped
    assert.match(await text(), /250\.5 USDC/);
    await evaluate('document.activeElement.blur()');
    await until("document.body.innerText.includes('251.5 USDC')", 'older result still applied');
  });

  test('an unconfirmed error leaves the button disabled the whole time', async () => {
    await open('send-unconfirmed', 'send');
    await until("document.querySelector('input[name=payee]')", 'payee list');
    await fill('input[type=text]', '5');
    await evaluate(`(() => {
      const b = [...document.querySelectorAll('button')].find((e) => e.textContent.trim() === 'Отправить');
      window.__log = [];
      new MutationObserver((rs) => rs.forEach((r) => window.__log.push(r.oldValue === null ? 'disabled' : 'enabled')))
        .observe(b, { attributes: true, attributeFilter: ['disabled'], attributeOldValue: true });
    })()`);
    await click('Отправить');
    await until("document.body.innerText.includes('Не закончена операция')", 'warning');
    await sleep(300);
    assert.equal(await buttonDisabled('Отправить'), true);
    assert.deepEqual(await evaluate('window.__log'), ['disabled']);
  });

  test('names from the chain are shown as text, never as HTML', async () => {
    await open('owner', 'people');
    await until("document.body.innerText.includes('Внучка Анна')", 'people list');
    await evaluate(`import('./dom.js').then(({h}) => { document.body.append(h('div', {id:'x'}, '<img src=x onerror=window.__pwned=1>')); })`);
    await sleep(200);
    assert.equal(await evaluate('window.__pwned === undefined && !document.querySelector("#x img")'), true);
  });

  for (const [name, fn] of tests) {
    try {
      await fn();
      console.log(`ok   ${name}`);
    } catch (e) {
      failed++;
      console.log(`FAIL ${name}\n     ${String(e.message).split('\n').join('\n     ')}`);
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
