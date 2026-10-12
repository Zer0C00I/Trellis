// Plays the whole scenario live on demo/stage.html (real app in an iframe, a moving cursor, typed text,
// finger taps on the kiosk, a pulse on the link) and records it with the DevTools screencast.
import { execFileSync } from 'node:child_process';
import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { scenes, APP_URL } from './scenario.mjs';
import { launch, serve } from './cdp.mjs';
import { setLocale, t } from '../family-app/ui/i18n.js';

setLocale('en', { persist: false });
rmSync('demo/out/rec', { recursive: true, force: true });
mkdirSync('demo/out/rec', { recursive: true });

const server = await serve('.');
const page = await launch({ width: 1920, height: 1080 });
const frames = [];
let writing = Promise.resolve();
page.on('Page.screencastFrame', (p) => {
  const file = `demo/out/rec/${String(frames.length).padStart(6, '0')}.jpg`;
  frames.push({ file, ts: p.metadata.timestamp });
  writing = writing.then(() => import('node:fs/promises').then((fs) => fs.writeFile(file, Buffer.from(p.data, 'base64'))));
  page.send('Page.screencastFrameAck', { sessionId: p.sessionId });
});

const js = (value) => JSON.stringify(value);
const run = (code) => page.evaluate(`(async () => { ${code} })()`);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

try {
  await page.navigate(`${server.url}/demo/stage.html`);
  await run(`await stage.load(${js(`${server.url}/family-app/ui/index.html${APP_URL}`)});`);
  await page.send('Page.startScreencast', { format: 'jpeg', quality: 80, everyNthFrame: 1 });
  const started = Date.now();

  for (const s of scenes) {
    const t0 = Date.now();
    await run(`stage.caption(${js(s.caption)}); stage.link(${js(s.link)});`);
    const beats = s.kiosk.map(async (b, i) => {
      const show = b.screen ? `stage.show(${js(`${server.url}/demo/out/kiosk/${s.id}-${i}.png`)});` : '';
      if (b.at === 0) return run(show);
      if (b.tap) {
        await sleep(Math.max(0, b.at * 1000 - 500 - (Date.now() - t0)));
        await run(`stage.tap(${b.tap.x}, ${b.tap.y});`);
      }
      await sleep(Math.max(0, b.at * 1000 - (Date.now() - t0)));
      if (show) await run(show);
    });
    for (const step of s.app) {
      if (step.do === 'fill') await run(`await stage.fill(${js(step.selector)}, ${js(step.value)});`);
      else if (step.do === 'select') await run(`await stage.select(${js(step.selector)}, ${js(step.value)});`);
      else if (step.do === 'click') await run(`await stage.click(${js(t(step.key, step.params))});`);
      else if (step.do === 'wait') await run(`await stage.wait(${js(t(step.key, step.params))});`);
      else if (step.do === 'pause') await sleep(step.ms);
    }
    const used = Date.now() - t0;
    if (used > s.seconds * 1000) console.warn(`scene ${s.id} took ${(used / 1000).toFixed(1)} s, planned ${s.seconds} s`);
    await sleep(Math.max(0, s.seconds * 1000 - used));
    await Promise.all(beats);
    console.log('scene', s.id, ((Date.now() - started) / 1000).toFixed(1), 's');
  }
  await sleep(300);
  await page.send('Page.stopScreencast');
  await writing;
  const end = Math.max(frames.at(-1).ts + 0.5, frames[0].ts + scenes.reduce((n, s) => n + s.seconds, 0));
  const list = frames.map((f, i) => `file '${f.file.replace('demo/out/', '')}'\nduration ${((frames[i + 1]?.ts ?? end) - f.ts).toFixed(4)}`).join('\n');
  // ffmpeg's concat demuxer drops the last duration unless the file is listed once more.
  writeFileSync('demo/out/rec.txt', `${list}\nfile '${frames.at(-1).file.replace('demo/out/', '')}'\n`);
} finally {
  await page.close();
  server.close();
}

execFileSync('ffmpeg', ['-loglevel', 'error', '-y', '-f', 'concat', '-safe', '0', '-i', 'demo/out/rec.txt',
  '-vf', 'fps=30,format=yuv420p', '-c:v', 'libx264', '-crf', '20', 'demo/out/demo.mp4'], { stdio: 'inherit' });
rmSync('demo/out/rec', { recursive: true, force: true });
console.log('demo/out/demo.mp4');
