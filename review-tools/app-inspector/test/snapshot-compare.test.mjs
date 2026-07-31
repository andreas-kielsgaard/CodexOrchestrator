import assert from 'node:assert/strict';
import test from 'node:test';

import { compareSnapshots } from '../src/snapshot-compare.mjs';

function snapshot(fingerprint, screenshot = 'shot-a', pid = 42) {
  return {
    schemaVersion: 'review-app-observation/v1',
    application: {
      durableState: { value: { fingerprint, counts: { drafts: fingerprint === 'a' ? 1 : 2 } } },
      screenshot: { value: { sha256: screenshot } },
      process: { value: { pid } },
    },
  };
}

test('reports unchanged observations', () => {
  const result = compareSnapshots(snapshot('a'), snapshot('a'));
  assert.equal(result.changed, false);
  assert.equal(result.summary.durableStateChanged, false);
});

test('separates durable, screenshot, and process changes', () => {
  const result = compareSnapshots(snapshot('a'), snapshot('b', 'shot-b', 43));
  assert.equal(result.changed, true);
  assert.equal(result.summary.durableStateChanged, true);
  assert.equal(result.summary.screenshotChanged, true);
  assert.equal(result.summary.processChanged, true);
  assert.ok(result.changes.some((change) => change.path.endsWith('.counts.drafts')));
});
