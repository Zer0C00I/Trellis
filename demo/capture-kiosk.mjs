import { execFileSync } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import { scenes } from './scenario.mjs';

mkdirSync('demo/out/kiosk', { recursive: true });
for (const s of scenes.filter((x) => x.kiosk)) {
  const ppm = `demo/out/kiosk/${s.id}.ppm`;
  const extra = [s.kiosk.name && `name=${s.kiosk.name}`, s.kiosk.amount && `amount=${s.kiosk.amount}`].filter(Boolean);
  execFileSync('cargo', ['run', '-q', '-p', 'kiosk-gui', '--example', 'screens', '--', s.kiosk.screen, 'en', ...extra, ppm], { stdio: 'inherit' });
  execFileSync('ffmpeg', ['-loglevel', 'error', '-y', '-i', ppm, `demo/out/kiosk/${s.id}.png`]);
}
