import { execFileSync } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import { scenes } from './scenario.mjs';

mkdirSync('demo/out/kiosk', { recursive: true });
for (const s of scenes) {
  s.kiosk.forEach((b, i) => {
    if (!b.screen) return;
    const ppm = `demo/out/kiosk/${s.id}-${i}.ppm`;
    const extra = [b.name && `name=${b.name}`, b.amount && `amount=${b.amount}`, b.familyBalance && `family-balance=${b.familyBalance}`].filter(Boolean);
    execFileSync('cargo', ['run', '-q', '-p', 'kiosk-gui', '--example', 'screens', '--', b.screen, 'en', ...extra, ppm], { stdio: 'inherit' });
    execFileSync('ffmpeg', ['-loglevel', 'error', '-y', '-i', ppm, `demo/out/kiosk/${s.id}-${i}.png`]);
  });
}
