import assert from 'node:assert/strict';
import { Buffer } from 'node:buffer';
import test from 'node:test';

import { parseInteractionOutput } from '../interact-app.mjs';

const prefix = 'REVIEW_APP_INTERACTION_V1:';
const frame = (value) =>
  `${prefix}${Buffer.from(JSON.stringify(value), 'utf8').toString('base64')}`;

test('parses one interaction receipt while ignoring incidental native output', () => {
  const expected = { transport: { foregrounded: false, delivery: 'window_messages_acknowledged' } };
  assert.deepEqual(parseInteractionOutput(`banner\n${frame(expected)}\ntrailer`), expected);
});

test('rejects missing and duplicate interaction receipts', () => {
  assert.throws(() => parseInteractionOutput('none'), /observed 0/u);
  const value = frame({ transport: {} });
  assert.throws(() => parseInteractionOutput(`${value}\n${value}`), /observed 2/u);
});
