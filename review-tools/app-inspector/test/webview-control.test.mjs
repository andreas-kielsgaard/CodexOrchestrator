import assert from 'node:assert/strict';
import test from 'node:test';

import { clickExpression, loopbackUrl, typeExpression } from '../webview-control.mjs';

test('keeps selector and text literal in CDP expressions', () => {
  const expression = typeExpression('textarea[data-id="draft"]', 'line one\nline two');
  assert.match(expression, /textarea\[data-id=\\"draft\\"\]/u);
  assert.match(expression, /line one\\nline two/u);
  assert.match(clickExpression('button[type="submit"]'), /button\[type=\\"submit\\"\]/u);
});

test('accepts only loopback debugger URLs', () => {
  assert.equal(loopbackUrl('http://127.0.0.1:9225/'), 'http://127.0.0.1:9225');
  assert.equal(loopbackUrl('https://localhost:9225'), 'https://localhost:9225');
  assert.throws(() => loopbackUrl('http://example.com:9225'), /loopback/u);
});
