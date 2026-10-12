import test from 'node:test';
import assert from 'node:assert/strict';
import { normalizeError } from '../errors.js';
import { classifyPairingInput } from '../input.js';
import { formatTime } from '../format.js';
import { parseRoute, ROUTES } from '../router.js';
import { setLocale } from '../i18n.js';

test('an ErrorDto from Rust passes through unchanged', () => {
  const dto = { kind: 'rejected', key: 'error.rejected_generic', params: {}, details: 'x' };
  assert.deepEqual(normalizeError(dto), dto);
});

test('an ErrorDto without params gets empty ones', () => {
  assert.deepEqual(normalizeError({ kind: 'blocked', key: 'error.busy' }).params, {});
  assert.equal(normalizeError({ kind: 'blocked', key: 'error.busy', params: [1] }).params.length, undefined);
});

test('an old-shaped error (a ready sentence, no key) is not trusted as a catalog key', () => {
  const e = normalizeError({ kind: 'rejected', message: 'Не получилось.' });
  assert.equal(e.kind, 'internal');
  assert.equal(e.key, 'error.generic');
});

test('anything else becomes an internal error with the original text as details', () => {
  const e = normalizeError(new Error('boom'));
  assert.equal(e.kind, 'internal');
  assert.equal(e.details, 'boom');
  assert.equal(e.key, 'error.generic');
  assert.equal(normalizeError('plain').details, 'plain');
  assert.equal(normalizeError(undefined).kind, 'internal');
});

test('pairing input: six digits is a code, fvpair1 text is a QR, the rest is not', () => {
  assert.deepEqual(classifyPairingInput(' 123456 '), { kind: 'pin', value: '123456' });
  assert.deepEqual(classifyPairingInput('fvpair1:00112233445566778899aabbccddeeff'), {
    kind: 'qr',
    value: 'fvpair1:00112233445566778899aabbccddeeff',
  });
  assert.equal(classifyPairingInput('').kind, 'empty');
  assert.equal(classifyPairingInput(null).kind, 'empty');
  assert.equal(classifyPairingInput('12345').kind, 'unknown');
  assert.equal(classifyPairingInput('1234567').kind, 'unknown');
  assert.equal(classifyPairingInput('hello').kind, 'unknown');
});

const NOW = Date.UTC(2026, 9, 9, 15, 0, 0); // 9 Oct 2026, 15:00 UTC
const at = (y, mo, d, h, mi) => Date.UTC(y, mo - 1, d, h, mi) / 1000;

test('time in Russian: today, earlier this year, other year, unknown', () => {
  setLocale('ru', { persist: false });
  assert.equal(formatTime(at(2026, 10, 9, 14, 32), NOW, 'UTC'), 'сегодня, 14:32');
  assert.equal(formatTime(at(2026, 10, 5, 9, 5), NOW, 'UTC'), '5 окт., 09:05');
  assert.equal(formatTime(at(2025, 12, 31, 23, 59), NOW, 'UTC'), '31 дек. 2025, 23:59');
  assert.equal(formatTime(null, NOW, 'UTC'), '');
  assert.equal(formatTime(undefined, NOW, 'UTC'), '');
});

test('time in English: today, earlier this year, other year', () => {
  setLocale('en', { persist: false });
  assert.equal(formatTime(at(2026, 10, 9, 14, 32), NOW, 'UTC'), 'today, 14:32');
  assert.equal(formatTime(at(2026, 10, 5, 9, 5), NOW, 'UTC'), 'Oct 5, 09:05');
  assert.equal(formatTime(at(2025, 12, 31, 23, 59), NOW, 'UTC'), 'Dec 31, 2025, 23:59');
});

test('time: midnight is 00:00, not 24:00', () => {
  setLocale('en', { persist: false });
  assert.equal(formatTime(at(2026, 10, 9, 0, 0), NOW, 'UTC'), 'today, 00:00');
});

test('routes: known names pass, anything else is home', () => {
  for (const r of ROUTES) assert.equal(parseRoute(`#/${r}`), r);
  assert.equal(parseRoute(''), 'home');
  assert.equal(parseRoute('#/nope'), 'home');
  assert.equal(parseRoute('#/send?x=1'), 'send');
  assert.equal(parseRoute(undefined), 'home');
});
