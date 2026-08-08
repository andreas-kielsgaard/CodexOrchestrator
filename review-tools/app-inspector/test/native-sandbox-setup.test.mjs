import assert from 'node:assert/strict';
import test from 'node:test';
import { setupArguments, setupEnvironment } from '../native-sandbox-setup.mjs';

test('binds the setup request to the retained selected profile and official argument vector', () => {
  assert.deepEqual(setupArguments(), ['sandbox', 'setup', '--elevated', '--current-user', '--codex-home', 'C:\\Users\\user\\.codex']);
  assert.throws(() => setupArguments('C:\\other'));
});

test('clears inherited state except the fixed Windows launch environment and selected CODEX_HOME', () => {
  const environment = setupEnvironment(undefined, { PATH: 'p', SYSTEMROOT: 's', TOKEN: 'must-not-pass' });
  assert.deepEqual(environment, { CODEX_HOME: 'C:\\Users\\user\\.codex', PATH: 'p', SYSTEMROOT: 's' });
});
