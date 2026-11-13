import test from 'node:test';
import assert from 'node:assert/strict';
import { effectiveElapsedMs, formatElapsed } from '../src/lib/focusTime.ts';

test('timer remains bounded after sleep, crash or clock rollback', () => {
  assert.equal(effectiveElapsedMs(15000, 1000, 12000, true), 26000);
  assert.equal(effectiveElapsedMs(15000, 1000, 3_600_000, true), 35000);
  assert.equal(effectiveElapsedMs(15000, 3000, 1000, true), 15000);
  assert.equal(effectiveElapsedMs(15000, 3000, 100000, false), 15000);
  assert.equal(effectiveElapsedMs(-9000, NaN, Infinity, true), 0);
});
test('elapsed duration is accessible, stable and readable', () => {
  assert.equal(formatElapsed(0), '00:00');
  assert.equal(formatElapsed(61000), '01:01');
  assert.equal(formatElapsed(3_661_000), '01:01:01');
  assert.equal(formatElapsed(-1000), '00:00');
  assert.equal(formatElapsed(Number.NaN), '00:00');
});
