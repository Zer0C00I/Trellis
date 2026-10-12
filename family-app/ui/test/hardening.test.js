import test from 'node:test';
import assert from 'node:assert/strict';
import { normalizeError } from '../errors.js';
import { classifyPairingInput } from '../input.js';
import { formatTime } from '../format.js';
import { ROUTES } from '../router.js';

const NOW = Date.UTC(2026, 9, 9, 15, 0, 0);
const at = (y, mo, d, h, mi) => Date.UTC(y, mo - 1, d, h, mi) / 1000;

test('formatTime: invalid input yields empty string, never throws', () => {
  for (const bad of [NaN, Infinity, -Infinity, 'abc', {}, null, true, [], 8.64e15, 1.7e12]) {
    assert.equal(formatTime(bad, NOW, 'UTC'), '', String(bad));
  }
});

test('formatTime: NaN nowMs falls back, invalid timeZone gives empty', () => {
  assert.equal(formatTime(at(2026, 10, 9, 14, 32), NaN, 'UTC').endsWith('14:32'), true);
  assert.equal(formatTime(at(2026, 10, 9, 14, 32), 'x', 'UTC').endsWith('14:32'), true);
  assert.equal(formatTime(at(2026, 10, 9, 14, 32), NOW, 'Not/AZone'), '');
});

test('normalizeError never throws on hostile values', () => {
  const nullProto = Object.create(null);
  const getter = { get message() { throw new Error('g'); } };
  const badStr = { toString() { throw new Error('t'); } };
  for (const v of [nullProto, getter, badStr, Symbol('s')]) {
    const e = normalizeError(v);
    assert.equal(e.kind, 'internal');
    assert.equal(typeof e.details, 'string');
  }
  assert.equal(normalizeError(badStr).details, 'unknown error');
});

test('normalizeError: empty key is wrapped, details coerced to string', () => {
  const e = normalizeError({ kind: 'rejected', key: '', details: 'd' });
  assert.equal(e.kind, 'internal');
  assert.equal(e.key, 'error.generic');
  const c = normalizeError({ kind: 'rejected', key: 'error.busy', details: 42 });
  assert.equal(c.kind, 'rejected');
  assert.equal(c.details, '42');
});

test('pairing input: whitespace in code, case-insensitive prefix, payload, length', () => {
  assert.deepEqual(classifyPairingInput('123 456'), { kind: 'pin', value: '123456' });
  assert.deepEqual(classifyPairingInput('FVPair1:abcd'), { kind: 'qr', value: 'fvpair1:abcd' });
  assert.equal(classifyPairingInput('fvpair1:').kind, 'unknown');
  assert.equal(classifyPairingInput('fvpair1:' + 'a'.repeat(57)).kind, 'unknown');
  assert.equal(classifyPairingInput('fvpair1:' + 'a'.repeat(56)).kind, 'qr');
});

test('ROUTES is frozen', () => {
  assert.ok(Object.isFrozen(ROUTES));
});

test('normalizeError never throws, even on throwing getters and proxies', () => {
  const trap = () => { throw new Error('trap'); };
  const proxy = new Proxy({}, { get: trap, has: trap, getPrototypeOf: trap, ownKeys: trap, getOwnPropertyDescriptor: trap });
  const cases = [
    { kind: 'k', key: 'error.busy', get details() { throw 1; } },
    { get kind() { throw 1; }, key: 'error.busy' },
    { kind: 'k', get key() { throw 1; } },
    proxy,
  ];
  for (const c of cases) {
    const e = normalizeError(c);
    assert.equal(e.kind, 'internal');
    assert.equal(e.details, 'unknown error');
    assert.equal(e.key, 'error.generic');
  }
});
