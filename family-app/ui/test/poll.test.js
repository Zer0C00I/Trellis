import test from 'node:test';
import assert from 'node:assert/strict';
import { waitingDelayMs } from '../poll.js';

test('waitingDelayMs backs off in three steps', () => {
  for (const n of [0, 1, 5]) assert.equal(waitingDelayMs(n), 5000);
  for (const n of [6, 7, 11]) assert.equal(waitingDelayMs(n), 10000);
  for (const n of [12, 13, 1000]) assert.equal(waitingDelayMs(n), 30000);
});

test('waitingDelayMs treats bad input as attempt 0 and floors fractions', () => {
  for (const n of [-1, -100, NaN, undefined, null, 'x', -Infinity]) assert.equal(waitingDelayMs(n), 5000);
  assert.equal(waitingDelayMs(5.9), 5000);
  assert.equal(waitingDelayMs(6.1), 10000);
  assert.equal(waitingDelayMs(Infinity), 30000);
});
