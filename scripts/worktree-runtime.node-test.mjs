import assert from 'node:assert/strict';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import test from 'node:test';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import {
  assertSafeToReprepare,
  isolatedEnvironment,
  keyedHash,
  portsForSlot,
  sourceState,
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
  assert.equal(result.environment.VITE_RUNTIME_INSTANCE_ID, 'alpha');
  assert.equal(result.environment.VITE_RUNTIME_BUILD_OBSERVED, 'false');
  assert.deepEqual(result.scrubbed, ['GITHUB_TOKEN', 'OPENAI_API_KEY']);
});

test('allows re-prepare only after the recorded instance is cleanly stopped', () => {
  assert.doesNotThrow(() =>
    assertSafeToReprepare(
      {
        processes: [],
        health: { status: { ok: false }, vite: { ok: false } },
        applicationProcessObserved: false,
        stale: false,
      },
      'alpha',
    ),
  );
});

test('refuses re-prepare for live, unowned, endpoint-only, or stale launch state', () => {
  const stopped = {
    processes: [],
    health: { status: { ok: false }, vite: { ok: false } },
    applicationProcessObserved: false,
    stale: false,
  };
  assert.throws(
    () =>
      assertSafeToReprepare(
        { ...stopped, processes: [{ pid: 10, alive: true, owned: true }] },
        'alpha',
      ),
    /must be stopped first/,
  );
  assert.throws(
    () =>
      assertSafeToReprepare(
        { ...stopped, processes: [{ pid: 11, alive: true, owned: false }] },
        'alpha',
      ),
    /ownership is unproven/,
  );
  assert.throws(
    () =>
      assertSafeToReprepare(
        { ...stopped, health: { status: { ok: true }, vite: { ok: false } } },
        'alpha',
      ),
    /runtime endpoint is live/,
  );
  assert.throws(
    () => assertSafeToReprepare({ ...stopped, stale: true }, 'alpha'),
    /must be recovered first/,
  );
});

test('nested untracked file content changes invalidate the source fingerprint', async () => {
  const workspace = await mkdtemp(path.join(os.tmpdir(), 'worktree-runtime-source-state-'));
  try {
    runGit(workspace, 'init');
    await writeFile(path.join(workspace, 'tracked.txt'), 'baseline\n', 'utf8');
    runGit(workspace, 'add', 'tracked.txt');
    runGit(
      workspace,
      '-c',
      'user.name=Worktree Runtime Test',
      '-c',
      'user.email=worktree-runtime@example.invalid',
      'commit',
      '-m',
      'baseline',
    );
    const commit = runGit(workspace, 'rev-parse', 'HEAD');
    const nested = path.join(workspace, 'untracked', 'nested', 'proof.txt');
    await mkdir(path.dirname(nested), { recursive: true });
    await writeFile(nested, 'first\n', 'utf8');
    const first = await sourceState(workspace, commit);

    await writeFile(nested, 'second\n', 'utf8');
    const second = await sourceState(workspace, commit);

    assert.equal(first.dirty, true);
    assert.equal(second.dirty, true);
    assert.notEqual(first.fingerprint, second.fingerprint);
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
});

function runGit(workspace, ...args) {
  const result = spawnSync('git', ['-C', workspace, ...args], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  return result.stdout.trim();
}
