import { mkdirSync } from 'node:fs';
import { scenes, APP_URL } from './scenario.mjs';
import { launch, serve } from './cdp.mjs';
import { setLocale, t } from '../family-app/ui/i18n.js';

setLocale('en', { persist: false });
mkdirSync('demo/out/app', { recursive: true });
const server = await serve('family-app/ui');
const page = await launch({ width: 390, height: 780, scale: 2 });
try {
  await page.navigate(`${server.url}/index.html${APP_URL}`);
  await page.until("document.getElementById('app').innerText.length > 20", 'the app to render');
  for (const s of scenes) {
    for (const step of s.app) {
      if (step.do === 'fill') await page.fill(step.selector, step.value);
      else if (step.do === 'select') await page.select(step.selector, step.value);
      else if (step.do === 'click') await page.click(t(step.key));
      else if (step.do === 'wait') await page.until(`document.body.innerText.includes(${JSON.stringify(t(step.key))})`, t(step.key), 60000);
      else if (step.do === 'pause') await page.sleep(step.ms);
    }
    await page.sleep(400);
    await page.screenshot(`demo/out/app/${s.id}.png`);
    console.log('app', s.id);
  }
} finally {
  await page.close();
  server.close();
}
