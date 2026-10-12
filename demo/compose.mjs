import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { scenes } from './scenario.mjs';
import { launch, serve } from './cdp.mjs';

mkdirSync('demo/out/frames', { recursive: true });
writeFileSync('demo/out/compose.html', `<!doctype html><meta charset="utf-8">
<style>
  html,body{margin:0;width:1920px;height:1080px;background:#0f1419;color:#fff;font-family:system-ui,sans-serif}
  .stage{position:absolute;inset:0 0 170px 0;display:flex;gap:80px;align-items:center;justify-content:center}
  .label{font-size:26px;opacity:.7;margin-bottom:14px;text-align:center}
  .kiosk img{width:880px;height:528px;border:6px solid #fff;border-radius:18px;display:block;background:#fff}
  .phone img{width:400px;height:800px;border:6px solid #fff;border-radius:34px;display:block;background:#fff}
  .link{width:340px;text-align:center}
  .link .via{display:inline-block;padding:6px 18px;border-radius:999px;border:2px solid #4fd1a5;color:#4fd1a5;font-size:26px}
  .link .arrow{font-size:96px;line-height:1.1;margin:10px 0}
  .link .what{font-size:28px;line-height:1.3;opacity:.9}
  .link.none .arrow{opacity:.25}
  .caption{position:absolute;left:0;right:0;bottom:0;height:170px;display:flex;align-items:center;justify-content:center;
    padding:0 120px;font-size:40px;line-height:1.3;text-align:center;background:#000}
</style>
<div class="stage" id="stage"></div><div class="caption" id="caption"></div>
<script>
  const p = new URLSearchParams(location.search);
  const part = (cls, label, src) => src ? '<div class="' + cls + '"><div class="label">' + label + '</div><img src="' + src + '"></div>' : '';
  const arrows = { right: '\u2192', left: '\u2190', both: '\u2194' };
  const link = p.get('via')
    ? '<div class="link"><div class="via">' + p.get('via') + '</div><div class="arrow">' + arrows[p.get('dir')] + '</div><div class="what">' + p.get('what') + '</div></div>'
    : '<div class="link none"><div class="via" style="visibility:hidden">-</div><div class="arrow">\u2194</div><div class="what">not connected yet</div></div>';
  document.getElementById('stage').innerHTML = part('kiosk', 'Kiosk (Rust, Slint)', p.get('kiosk')) + link + part('phone', 'Family app (Tauri)', p.get('app'));
  document.getElementById('caption').textContent = p.get('caption');
  Promise.all([...document.images].map((i) => i.decode().catch(() => {}))).then(() => { document.title = 'ready'; });
</script>`);

const server = await serve('demo/out');
const page = await launch({ width: 1920, height: 1080 });
try {
  for (const s of scenes) {
    const q = new URLSearchParams({ caption: s.caption });
    if (s.kiosk) q.set('kiosk', `kiosk/${s.id}.png`);
    if (s.link) { q.set('via', s.link.via); q.set('dir', s.link.dir); q.set('what', s.link.label); }
    if (existsSync(`demo/out/app/${s.id}.png`)) q.set('app', `app/${s.id}.png`);
    await page.navigate(`${server.url}/compose.html?${q}`);
    await page.until("document.title === 'ready'", `frame ${s.id}`, 15000);
    await page.screenshot(`demo/out/frames/${s.id}.png`);
  }
} finally {
  await page.close();
  server.close();
}

const list = scenes.map((s) => `file 'frames/${s.id}.png'\nduration ${s.seconds}`).join('\n');
writeFileSync('demo/out/frames.txt', `${list}\nfile 'frames/${scenes.at(-1).id}.png'\n`);
execFileSync('ffmpeg', ['-loglevel', 'error', '-y', '-f', 'concat', '-i', 'demo/out/frames.txt',
  '-t', String(scenes.reduce((n, s) => n + s.seconds, 0)), '-vf', 'fps=30,format=yuv420p', '-c:v', 'libx264', 'demo/out/demo.mp4'], { stdio: 'inherit' });
console.log('demo/out/demo.mp4');
