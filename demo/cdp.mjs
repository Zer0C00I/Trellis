// Minimal headless-Chromium driver over the DevTools protocol, no packages.
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { readFile, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { extname, join, normalize } from 'node:path';

const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.css': 'text/css', '.png': 'image/png' };

export async function serve(root) {
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
  return { url: `http://127.0.0.1:${server.address().port}`, close: () => server.close() };
}

export async function launch({ width, height, scale = 1 }) {
  const profile = await mkdtemp(join(tmpdir(), 'demo-'));
  const chrome = spawn(process.env.CHROMIUM || 'chromium', [
    '--headless=new', '--no-sandbox', '--disable-gpu', '--hide-scrollbars', '--remote-debugging-port=0',
    `--user-data-dir=${profile}`, 'about:blank',
  ], { stdio: 'ignore' });
  const died = new Promise((_, reject) => {
    chrome.once('error', (e) => reject(new Error(`cannot start Chromium: ${e.message}`)));
    chrome.once('exit', (code) => reject(new Error(`Chromium exited during startup (code ${code})`)));
  });
  died.catch(() => {});

  async function endpoint() {
    for (let i = 0; i < 300; i++) {
      try {
        const port = (await readFile(join(profile, 'DevToolsActivePort'), 'utf8')).split('\n')[0].trim();
        const page = (await (await fetch(`http://127.0.0.1:${port}/json`)).json()).find((t) => t.type === 'page');
        if (page) return page.webSocketDebuggerUrl;
      } catch { /* not up yet */ }
      await new Promise((r) => setTimeout(r, 100));
    }
    throw new Error('Chromium did not start within 30 s');
  }

  const ws = new WebSocket(await Promise.race([endpoint(), died]));
  await new Promise((r, j) => { ws.onopen = r; ws.onerror = () => j(new Error('DevTools socket failed')); });
  let id = 0;
  const pending = new Map();
  const loaders = [];
  const listeners = new Map();
  ws.onmessage = ({ data }) => {
    const m = JSON.parse(data);
    if (m.id && pending.has(m.id)) pending.get(m.id)(m);
    if (m.method === 'Page.loadEventFired') loaders.splice(0).forEach((r) => r());
    if (m.method) listeners.get(m.method)?.(m.params);
  };
  const send = (method, params = {}) => new Promise((r) => { pending.set(++id, r); ws.send(JSON.stringify({ id, method, params })); });
  await send('Page.enable');
  await send('Runtime.enable');
  await send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: scale, mobile: false });

  const evaluate = async (expression) => {
    const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
    if (r.result.exceptionDetails) throw new Error(r.result.exceptionDetails.exception?.description ?? 'evaluate failed');
    return r.result.result.value;
  };
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  async function until(expression, what, ms = 30000) {
    const end = Date.now() + ms;
    while (Date.now() < end) {
      if (await evaluate(expression)) return;
      await sleep(100);
    }
    throw new Error(`timed out waiting for: ${what}\npage text: ${await evaluate('document.body.innerText')}`);
  }
  const navigate = (url) => new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`no load event for ${url}`)), 15000);
    loaders.push(() => { clearTimeout(timer); resolve(); });
    send('Page.navigate', { url });
  });
  const click = (label) => until(
    `(() => { const e = [...document.querySelectorAll('button,a')].find(e => e.textContent.trim().replace(/\\d+$/, '').trim() === ${JSON.stringify(label)}); if (!e) return false; e.click(); return true; })()`,
    `a button or link "${label}"`);
  const fill = (selector, value) => evaluate(
    `(() => { const e = document.querySelector(${JSON.stringify(selector)}); e.focus(); e.value = ${JSON.stringify(value)}; e.dispatchEvent(new Event('input', {bubbles:true})); })()`);
  const select = (selector, value) => evaluate(
    `(() => { const e = document.querySelector(${JSON.stringify(selector)}); e.value = ${JSON.stringify(value)}; e.dispatchEvent(new Event('change', {bubbles:true})); })()`);
  const screenshot = async (path) => {
    const r = await send('Page.captureScreenshot', { format: 'png' });
    await writeFile(path, Buffer.from(r.result.data, 'base64'));
  };
  const on = (method, fn) => listeners.set(method, fn);
  const close = async () => {
    ws.close();
    chrome.kill();
    await rm(profile, { recursive: true, force: true }).catch(() => {});
  };
  return { evaluate, until, navigate, click, fill, select, screenshot, sleep, send, on, close };
}
