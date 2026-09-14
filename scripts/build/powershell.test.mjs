import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

function wrapper(t, name, args, exitCode = 0) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'build wrapper spaces-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const scripts = path.join(root, 'scripts');
  fs.mkdirSync(scripts);
  fs.copyFileSync(fileURLToPath(new URL('../' + name, import.meta.url)), path.join(scripts, name));
  const record = path.join(root, 'call.json');
  fs.writeFileSync(
    path.join(scripts, 'build-tools.mjs'),
    [
      "import fs from 'node:fs';",
      'fs.writeFileSync(process.env.BUILD_TEST_RECORD, JSON.stringify({args:process.argv.slice(2),env:process.env}));',
      'process.exit(Number(process.env.BUILD_TEST_EXIT));',
    ].join('\n'),
  );
  const env = {
    ...process.env,
    BUILD_TEST_RECORD: record,
    BUILD_TEST_EXIT: String(exitCode),
    RUSTC_WRAPPER: 'caller-wrapper',
  };
  delete env.CARGO_TARGET_DIR;
  const result = spawnSync(
    'powershell.exe',
    ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', path.join(scripts, name), ...args],
    { encoding: 'utf8', env, windowsHide: true },
  );
  return { result, call: fs.existsSync(record) ? JSON.parse(fs.readFileSync(record)) : null, root };
}

test(
  'PowerShell shared helper forwards paths and harness arguments and preserves failure exit codes',
  { skip: process.platform !== 'win32' },
  (t) => {
    const { result, call, root } = wrapper(
      t,
      'cargo-sccache.ps1',
      ['test', '-TargetDir', 'cache with spaces', '--lib', 'filter', '--', '--nocapture'],
      37,
    );
    assert.equal(result.status, 37, result.stderr);
    assert.deepEqual(call.args, [
      'test',
      '--cache=shared',
      '--worktree',
      root,
      '--target-dir',
      'cache with spaces',
      '--lib',
      'filter',
      '--',
      '--nocapture',
    ]);
    assert.equal(call.env.RUSTC_WRAPPER, 'caller-wrapper');
  },
);

test(
  'PowerShell fast helper preserves its reduced-debug test semantics and explicit cache choice',
  { skip: process.platform !== 'win32' },
  (t) => {
    const { result, call, root } = wrapper(t, 'cargo-test-fast.ps1', [
      '-Cache',
      'local',
      '--lib',
      '--no-run',
      '--locked',
    ]);
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(call.args, [
      'test',
      '--cache=auto',
      '--test-debug=0',
      '--worktree',
      root,
      '--cache',
      'local',
      '--lib',
      '--no-run',
      '--locked',
    ]);
    assert.equal(call.env.RUSTC_WRAPPER, 'caller-wrapper');
  },
);

test(
  'PowerShell fast helper rejects a conflicting profile before invoking compilation',
  { skip: process.platform !== 'win32' },
  (t) => {
    const { result, call } = wrapper(t, 'cargo-test-fast.ps1', ['--profile', 'test-fast']);
    assert.notEqual(result.status, 0);
    assert.equal(call, null);
  },
);
