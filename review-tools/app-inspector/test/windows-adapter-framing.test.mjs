import assert from 'node:assert/strict';
import { Buffer } from 'node:buffer';
import test from 'node:test';

import { parseWindowsAdapterOutput } from '../src/windows-adapter.mjs';

const prefix = 'REVIEW_APP_JSON_V1:';

function frame(value) {
  return `${prefix}${Buffer.from(JSON.stringify(value), 'utf8').toString('base64')}`;
}

test('ignores incidental native output around one framed payload', () => {
  const expected = {
    process: { disposition: 'observed', value: { running: true, pid: 19760 } },
    screenshot: { disposition: 'observed', value: { sha256: 'abc' } },
  };
  const malformedRawJsonClass = `native banner\n{"process":{"value":"unterminated}\n${frame(expected)}\ntrailer`;

  assert.deepEqual(parseWindowsAdapterOutput(malformedRawJsonClass), expected);
});

test('rejects a missing framed payload', () => {
  assert.throws(
    () => parseWindowsAdapterOutput('{"process":{"value":"unterminated}'),
    /expected exactly one.*observed 0/u,
  );
});

test('rejects duplicate framed payloads', () => {
  const value = frame({ process: {} });
  assert.throws(() => parseWindowsAdapterOutput(`${value}\n${value}`), /observed 2/u);
});

test('rejects malformed base64 and malformed framed JSON', () => {
  assert.throws(() => parseWindowsAdapterOutput(`${prefix}%%%=`), /canonical base64/u);
  const malformedJson = `${prefix}${Buffer.from('{"process":', 'utf8').toString('base64')}`;
  assert.throws(() => parseWindowsAdapterOutput(malformedJson), /contains malformed JSON/u);
});
