import test from 'node:test';
import assert from 'node:assert/strict';
import { scenes, KIOSK_SCREENS } from '../scenario.mjs';
import { setLocale, t, hooks } from '../../family-app/ui/i18n.js';

const total = scenes.reduce((s, x) => s + x.seconds, 0);

test('scene ids are unique and every scene has a caption and a positive length', () => {
  assert.equal(new Set(scenes.map((s) => s.id)).size, scenes.length);
  for (const s of scenes) {
    assert.ok(s.caption.length > 10, s.id);
    assert.ok(s.seconds > 0, s.id);
  }
});

test('every kiosk screen exists in the screens example', () => {
  for (const s of scenes) {
    assert.ok(s.kiosk.length > 0 && s.kiosk[0].at === 0 && s.kiosk[0].screen, `${s.id}: starts with a screen at 0`);
    let last = -1;
    for (const b of s.kiosk) {
      assert.ok(b.at > last || b.at === 0, `${s.id}: beats are in time order`);
      assert.ok(b.at < s.seconds, `${s.id}: beat inside the scene`);
      if (b.screen) assert.ok(KIOSK_SCREENS.includes(b.screen), `${s.id}: ${b.screen}`);
      if (b.tap) assert.ok(b.tap.x >= 0 && b.tap.x <= 1 && b.tap.y >= 0 && b.tap.y <= 1, `${s.id}: tap inside the screen`);
      last = b.at;
    }
  }
});

test('every catalog key an app step uses exists in the English catalog', () => {
  setLocale('en', { persist: false });
  const missing = [];
  hooks.onMissing = (key) => missing.push(key);
  for (const s of scenes) for (const step of s.app) if (step.key) t(step.key);
  assert.deepEqual(missing, []);
});

test('app steps are known kinds with their fields', () => {
  for (const s of scenes) for (const step of s.app) {
    if (step.do === 'fill' || step.do === 'select') assert.ok(step.selector && step.value !== undefined, s.id);
    else if (step.do === 'click' || step.do === 'wait') assert.ok(step.key, s.id);
    else if (step.do === 'pause') assert.ok(step.ms > 0, s.id);
    else assert.fail(`${s.id}: unknown step ${step.do}`);
  }
});

test('the video is between 60 and 120 seconds', () => {
  assert.ok(total >= 60 && total <= 120, `total is ${total}`);
});

test('captions carry no on-screen disclaimer (it is added in the edit)', () => {
  for (const s of scenes) assert.doesNotMatch(s.caption, /simulated/i, s.id);
});

test('every link names its channel, a direction and says what travels', () => {
  for (const s of scenes) {
    if (s.link === null) continue;
    assert.ok(['Bluetooth LE', 'Solana devnet'].includes(s.link.via), s.id);
    assert.ok(['right', 'left', 'both'].includes(s.link.dir), s.id);
    assert.ok(s.link.label.length > 15, s.id);
  }
  assert.ok(scenes.every((s) => 'link' in s), 'every scene declares link (null when nothing is connected)');
});
