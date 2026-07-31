import assert from 'node:assert/strict';
import test from 'node:test';

import { waitForChange } from '../src/wait-for-change.mjs';

function harness(observations) {
  let clock = 0;
  let index = 0;
  return {
    now: () => clock,
    sleep: async (milliseconds) => {
      clock += milliseconds;
    },
    observe: async () => observations[Math.min(index++, observations.length - 1)],
  };
}

test('requires repeated stable visual change after a transient frame', async () => {
  const fake = harness([
    { visual: 'changed', durable: 'same' },
    { visual: 'baseline', durable: 'same' },
    { visual: 'changed', durable: 'same' },
    { visual: 'changed', durable: 'same' },
    { visual: 'changed', durable: 'same' },
  ]);
  const result = await waitForChange({
    baseline: { visual: 'baseline', durable: 'same' },
    condition: 'visual',
    pollMs: 100,
    stableObservations: 3,
    timeoutMs: 1_000,
    ...fake,
  });

  assert.equal(result.status, 'completed');
  assert.deepEqual(result.trigger, { kind: 'visual', value: 'changed' });
  assert.equal(result.observationCount, 5);
});

test('either condition remains stable on durable change despite render jitter', async () => {
  const fake = harness([
    { visual: 'frame-b', durable: 'state-b' },
    { visual: 'frame-c', durable: 'state-b' },
    { visual: 'frame-b', durable: 'state-b' },
  ]);
  const result = await waitForChange({
    baseline: { visual: 'frame-a', durable: 'state-a' },
    condition: 'either',
    pollMs: 100,
    stableObservations: 3,
    timeoutMs: 1_000,
    ...fake,
  });

  assert.equal(result.status, 'completed');
  assert.deepEqual(result.trigger, { kind: 'durable', value: 'state-b' });
  assert.equal(result.observationCount, 3);
});

test('returns a truthful timeout when no selected signal changes', async () => {
  const fake = harness([{ visual: 'same', durable: 'same' }]);
  const result = await waitForChange({
    baseline: { visual: 'same', durable: 'same' },
    condition: 'durable',
    pollMs: 100,
    stableObservations: 2,
    timeoutMs: 300,
    ...fake,
  });

  assert.equal(result.status, 'timed_out');
  assert.equal(result.observationCount, 2);
  assert.equal(result.stableCount, 0);
});

test('returns cancelled after an abort without claiming a change', async () => {
  const controller = new globalThis.AbortController();
  const fake = harness([{ visual: 'same', durable: 'same' }]);
  const result = await waitForChange({
    baseline: { visual: 'same', durable: 'same' },
    condition: 'either',
    pollMs: 100,
    stableObservations: 2,
    timeoutMs: 1_000,
    signal: controller.signal,
    now: fake.now,
    sleep: async (milliseconds) => {
      await fake.sleep(milliseconds);
      controller.abort();
    },
    observe: fake.observe,
  });

  assert.equal(result.status, 'cancelled');
  assert.equal(result.observationCount, 0);
});

test('returns cancelled when a detached cancellation signal appears', async () => {
  const fake = harness([{ visual: 'same', durable: 'same' }]);
  let checks = 0;
  const result = await waitForChange({
    baseline: { visual: 'same', durable: 'same' },
    condition: 'either',
    pollMs: 100,
    stableObservations: 2,
    timeoutMs: 1_000,
    isCancelled: async () => ++checks >= 2,
    ...fake,
  });

  assert.equal(result.status, 'cancelled');
  assert.equal(result.observationCount, 0);
});

test('rejects a condition whose baseline signal is unavailable', async () => {
  await assert.rejects(
    waitForChange({
      baseline: { visual: null, durable: 'state-a' },
      condition: 'visual',
      pollMs: 100,
      stableObservations: 2,
      timeoutMs: 1_000,
      observe: async () => ({ visual: null, durable: 'state-a' }),
    }),
    /no observed window-render hash/u,
  );
});
