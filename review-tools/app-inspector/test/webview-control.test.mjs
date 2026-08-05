import assert from 'node:assert/strict';
import { Buffer } from 'node:buffer';
import test from 'node:test';

import {
  clickExpression,
  debuggerPort,
  loopbackUrl,
  parseOwnershipOutput,
  typeExpression,
  validateWebSocketDebuggerUrl,
} from '../webview-control.mjs';

const ownershipPrefix = 'REVIEW_APP_WEBVIEW_OWNER_V1:';
const ownershipFrame = (value) =>
  `${ownershipPrefix}${Buffer.from(JSON.stringify(value), 'utf8').toString('base64')}`;

test('keeps selector and text literal in CDP expressions', () => {
  const expression = typeExpression('textarea[data-id="draft"]', 'line one\nline two');
  assert.match(expression, /textarea\[data-id=\\"draft\\"\]/u);
  assert.match(expression, /line one\\nline two/u);
  assert.match(clickExpression('button[type="submit"]'), /button\[type=\\"submit\\"\]/u);
});

test('accepts only explicit HTTP loopback debugger URLs', () => {
  assert.equal(loopbackUrl('http://127.0.0.1:9225/'), 'http://127.0.0.1:9225');
  assert.equal(debuggerPort('http://127.0.0.1:9225'), 9225);
  assert.throws(() => loopbackUrl('http://example.com:9225'), /loopback/u);
  assert.throws(() => loopbackUrl('https://localhost:9225'), /http loopback/u);
  assert.throws(() => loopbackUrl('http://localhost'), /explicit debugger port/u);
});

test('requires a loopback ws endpoint on the validated debugger port', () => {
  const debugUrl = 'http://127.0.0.1:9226';
  assert.equal(
    validateWebSocketDebuggerUrl('ws://127.0.0.1:9226/devtools/page/one', debugUrl),
    'ws://127.0.0.1:9226/devtools/page/one',
  );
  assert.throws(
    () => validateWebSocketDebuggerUrl('ws://example.test:9226/devtools/page/one', debugUrl),
    /host must be loopback/u,
  );
  assert.throws(
    () => validateWebSocketDebuggerUrl('ws://127.0.0.1:9227/devtools/page/one', debugUrl),
    /port must match/u,
  );
  assert.throws(
    () => validateWebSocketDebuggerUrl('wss://127.0.0.1:9226/devtools/page/one', debugUrl),
    /must expose a ws/u,
  );
});

test('parses one owned-debugger receipt and rejects malformed framing', () => {
  const expected = { owner: { pid: 7 }, debugger: { pid: 9, port: 9225 } };
  assert.deepEqual(parseOwnershipOutput(`native\n${ownershipFrame(expected)}\n`), expected);
  assert.throws(() => parseOwnershipOutput('none'), /observed 0/u);
  assert.throws(
    () => parseOwnershipOutput(`${ownershipFrame(expected)}\n${ownershipFrame(expected)}`),
    /observed 2/u,
  );
});
