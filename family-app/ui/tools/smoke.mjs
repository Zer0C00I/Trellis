// End-to-end smoke test of the screens against the mock backend, driving a
// real headless Chromium over the DevTools protocol (no extra packages).
//   node tools/smoke.mjs            (serves ui/ itself on a free port)
// Needs a Chromium binary: set CHROMIUM=/path if it is not "chromium".
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { readFile, mkdtemp } from 'node:fs/promises';
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
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const base = `http://127.0.0.1:${server.address().port}/index.html`;

const profile = await mkdtemp(join(tmpdir(), 'smoke-'));
const port = 9300 + Math.floor(Math.random() * 500);
const chrome = spawn(process.env.CHROMIUM || 'chromium', [
  '--headless=new', '--no-sandbox', '--disable-gpu', `--remote-debugging-port=${port}`,
  `--user-data-dir=${profile}`, '--window-size=420,900', 'about:blank',
], { stdio: 'ignore' });

async function connect() {
  for (let i = 0; i < 100; i++) {
    try {
      const list = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
      const page = list.find((t) => t.type === 'page');
      if (page) return page.webSocketDebuggerUrl;
    } catch { /* not up yet */ }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error('Chromium did not start');
}

const ws = new WebSocket(await connect());
await new Promise((r) => (ws.onopen = r));
let id = 0;
const pending = new Map();
const errors = [];
ws.onmessage = ({ data }) => {
  const m = JSON.parse(data);
  if (m.id && pending.has(m.id)) pending.get(m.id)(m);
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
async function until(expression, what, ms = 8000) {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    if (await evaluate(expression)) return;
    await sleep(100);
  }
  throw new Error(`timed out waiting for: ${what}\npage text: ${await evaluate('document.body.innerText')}`);
}
const open = async (scenario, route = 'home') => {
  await send('Page.navigate', { url: `${base}?mock=${scenario}#/${route}` });
  await until("document.getElementById('app').innerText.length > 20", `${scenario} to render`);
};
const text = () => evaluate('document.body.innerText');
const click = (label) => evaluate(`[...document.querySelectorAll('button,a')].find(e => e.textContent.trim() === ${JSON.stringify(label)})?.click()`);
const fill = (selector, value) => evaluate(`(() => { const e = document.querySelector(${JSON.stringify(selector)}); e.focus(); e.value = ${JSON.stringify(value)}; e.dispatchEvent(new Event('input', {bubbles:true})); })()`);

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
  await sleep(1200);
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

test('first launch: once the vault appears the main screen takes over', async () => {
  await open('waiting');
  await fill('input[autocomplete=name]', 'Внук Саша');
  await fill('input[placeholder^="fvpair1"]', '123456');
  await click('Передать устройству');
  await until("document.body.innerText.includes('Ждём, пока вас добавят')", 'waiting');
  await until("document.body.innerText.includes('Личный счёт')", 'vault shown', 30000);
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

test('a wrong fallback code shows the plain message and keeps the details folded', async () => {
  await open('empty');
  await fill('input[autocomplete=name]', 'Внук Саша');
  await fill('input[placeholder^="fvpair1"]', '000000');
  await click('Передать устройству');
  await until("document.body.innerText.includes('Устройство не приняло код')", 'rejection message');
  assert.equal(await evaluate("document.querySelector('details')?.open"), false);
});

test('an unconfirmed operation shows the warning until it is checked', async () => {
  await open('unconfirmed');
  await until("document.body.innerText.includes('Не закончена операция')", 'warning');
  await click('Проверить');
  await until("!document.body.innerText.includes('Не закончена операция')", 'warning cleared', 10000);
});

test('a background refresh never wipes what the person is typing', async () => {
  await open('owner', 'send');
  await until("document.querySelector('input[type=text]')", 'amount field');
  await fill('input[type=text]', '12.5');
  await evaluate("document.dispatchEvent(new Event('visibilitychange'))");
  await sleep(1500);
  assert.equal(await evaluate("document.querySelector('input[type=text]').value"), '12.5');
});

test('names from the chain are shown as text, never as HTML', async () => {
  await open('owner', 'people');
  await until("document.body.innerText.includes('Внучка Анна')", 'people list');
  await evaluate(`import('./dom.js').then(({h}) => { document.body.append(h('div', {id:'x'}, '<img src=x onerror=window.__pwned=1>')); })`);
  await sleep(200);
  assert.equal(await evaluate('window.__pwned === undefined && !document.querySelector("#x img")'), true);
});

let failed = 0;
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
chrome.kill();
server.close();
console.log(failed ? `${failed} failed` : 'all passed');
process.exit(failed ? 1 : 0);
