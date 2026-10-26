import assert from 'node:assert/strict';
import test from 'node:test';
import { createResetCreditLoader } from './codexResetCreditLoader';

test('reset details share in-flight requests and refresh after count or cache age changes', async () => {
  let calls = 0;
  let now = 0;
  const load = createResetCreditLoader(async () => {
    calls += 1;
    return { available_count: calls, credits: [], next_expires_at: 123 };
  }, () => now);
  const first = load('a', 1);
  assert.equal(load('a', 1), first);
  await first;
  await load('a', 1);
  assert.equal(calls, 1);
  await load('a', 2);
  assert.equal(calls, 2);
  now += 5 * 60_000;
  await load('a', 2);
  assert.equal(calls, 3);
  await load('b', 2);
  assert.equal(calls, 4);
});

test('failed reset detail requests can be retried', async () => {
  let calls = 0;
  const load = createResetCreditLoader(async () => {
    if (++calls === 1) throw new Error('offline');
    return { available_count: 1, credits: [] };
  });
  await assert.rejects(load('a', 1), /offline/);
  assert.equal((await load('a', 1)).available_count, 1);
  assert.equal(calls, 2);
});
