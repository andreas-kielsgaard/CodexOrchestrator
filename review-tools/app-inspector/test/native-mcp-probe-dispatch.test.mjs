import assert from 'node:assert/strict';
import test from 'node:test';
import { classifyProbe, fixedInvokeExpression } from '../native-mcp-probe-dispatch.mjs';

test('binds its only Tauri dispatch to the retained MCP probe command and profile', () => {
  const expression = fixedInvokeExpression();
  assert.match(expression, /probe_native_profile_mcp_reporting/);
  assert.match(expression, /native-profile-ed6f9ea9-f8a8-411b-ba0b-490859dc6126/);
  assert.doesNotMatch(expression, /reconcile_native_profile_mcp_reporting/);
});

test('classifies only pending or dispatching records as a current probe', () => {
  assert.deepEqual(classifyProbe([]), { disposition: 'absent' });
  assert.equal(classifyProbe([{ state: 'pending', requestedAt: 'a', deadlineAt: 'b' }]).disposition, 'current');
  assert.equal(classifyProbe([{ state: 'dispatching', requestedAt: 'a', deadlineAt: 'b' }]).disposition, 'current');
  assert.equal(classifyProbe([{ state: 'received', requestedAt: 'a', deadlineAt: 'b' }]).disposition, 'not_current');
});
