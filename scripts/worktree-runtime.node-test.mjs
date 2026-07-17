import assert from 'node:assert/strict';
import test from 'node:test';
import path from 'node:path';
import {
  isolatedEnvironment,
  keyedHash,
  portsForSlot,
  tauriOverride,
  validateInstanceId,
} from './worktree-runtime.mjs';

function manifest() {
  const root = path.resolve('.dev', 'worktree-runtime', 'alpha');
  return {
    identity: {
      instanceId: 'alpha',
      sessionId: 'session-alpha',
      worktreePath: path.resolve('.'),
      gitCommit: 'abc123',
      tauriIdentifier: 'dev.codex-orchestrator.worktree.abc123',
    },
    projected: {
      ports: { vite: 1440, status: 41435 },
      paths: {
        root,
        dist: path.join(root, 'dist'),
        cargoTarget: path.join(root, 'cargo-target'),
        appData: path.join(root, 'app-data'),
        credentials: path.join(root, 'credentials', 'codex-home'),
        runtimeStatus: path.join(root, 'runtime-status.json'),
      },
      caches: {
        node: { path: path.join(root, 'npm-cache') },
        rust: { mode: 'isolated-target-only', path: path.join(root, 'sccache') },
      },
    },
  };
}

test('validates instance identifiers and derives non-overlapping strict ports', () => {
  assert.equal(validateInstanceId('worker-01.alpha'), 'worker-01.alpha');
  assert.throws(() => validateInstanceId('../escape'));
  assert.deepEqual(portsForSlot(1), { vite: 1440, status: 41435 });
  assert.deepEqual(portsForSlot(2), { vite: 1460, status: 41455 });
});

test('cache keys change when any declared input changes', () => {
  assert.equal(keyedHash(['lock', 'node']), keyedHash(['lock', 'node']));
  assert.notEqual(keyedHash(['lock', 'node']), keyedHash(['lock-2', 'node']));
});

test('tauri override names the owner and isolates URL, frontend output, and bundle identity', () => {
  const config = tauriOverride(manifest());
  assert.equal(config.identifier, 'dev.codex-orchestrator.worktree.abc123');
  assert.equal(config.build.devUrl, 'http://127.0.0.1:1440');
  assert.equal(config.build.frontendDist, manifest().projected.paths.dist);
  assert.match(config.app.windows[0].title, /alpha/);
  assert.equal(config.bundle.active, false);
});

test('launch environment isolates instance state and removes ambient provider credentials', () => {
  const result = isolatedEnvironment(
    {
      PATH: 'example',
      OPENAI_API_KEY: 'secret',
      GITHUB_TOKEN: 'secret',
    },
    manifest(),
  );
  assert.equal(result.environment.PATH, 'example');
  assert.equal(result.environment.OPENAI_API_KEY, undefined);
  assert.equal(result.environment.GITHUB_TOKEN, undefined);
  assert.equal(result.environment.CODEX_HOME, manifest().projected.paths.credentials);
  assert.equal(
    result.environment.CODEX_ORCHESTRATOR_APP_DATA_DIR,
    manifest().projected.paths.appData,
  );
  assert.deepEqual(result.scrubbed, ['GITHUB_TOKEN', 'OPENAI_API_KEY']);
});
