import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import {
  acquireLock,
  cacheLayout,
  selectCache,
  inspectSharedCache,
  cargoEnvironment,
  invokeCargo,
  clearCache,
} from './cargo.mjs';
import {
  applicationBuildConfig,
  applicationSchemaVersion,
  publishApplication,
  frontendConfigPath,
} from './application.mjs';
import { run, BuildError } from './process.mjs';
import { parseArguments } from '../build-tools.mjs';

function fixture(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'co-build-test-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const worktree = path.join(root, 'checkout with spaces');
  fs.mkdirSync(path.join(worktree, 'src-tauri'), { recursive: true });
  const env = { ...process.env, ORCHESTRATOR_BUILD_CACHE_ROOT: path.join(root, 'cache') };
  delete env.CARGO_TARGET_DIR;
  return { root, worktree, env, layout: cacheLayout(worktree, undefined, env) };
}
const shared = { program: 'sccache', cacheDir: 'shared-cache', before: { stats: {} } };

test('auto chooses local profile artifacts before probing shared caching', (t) => {
  const { layout, env } = fixture(t);
  fs.mkdirSync(path.join(layout.target, 'debug', 'deps'), { recursive: true });
  fs.writeFileSync(path.join(layout.target, 'debug', 'deps', 'lib.rlib'), 'compiled');
  assert.equal(
    selectCache(layout, 'debug', 'auto', env, () => assert.fail('should not probe')).mode,
    'local',
  );
  assert.equal(selectCache(layout, 'release', 'auto', env, () => shared).mode, 'shared');
});

test('auto falls back when shared cache is unavailable; explicit shared reports the failure', (t) => {
  const { layout, env } = fixture(t);
  const missing = () => {
    throw new Error('not installed');
  };
  assert.match(selectCache(layout, 'release', 'auto', env, missing).reason, /not installed/);
  assert.throws(() => selectCache(layout, 'release', 'shared', env, missing), /not installed/);
});

test('the same worktree reuses a persistent target, including an existing ordinary Cargo target', (t) => {
  const { layout, worktree, env } = fixture(t);
  assert.equal(cacheLayout(worktree, undefined, env).target, layout.target);
  const ordinary = path.join(worktree, 'src-tauri', 'target');
  fs.mkdirSync(path.join(ordinary, 'test-fast', 'deps'), { recursive: true });
  fs.writeFileSync(path.join(ordinary, 'test-fast', 'deps', 'test.exe'), 'test');
  assert.equal(cacheLayout(worktree, undefined, env).target, ordinary);
});

test('local/shared environments are scoped and preserve the caller environment', () => {
  const env = {
    PATH: 'tools',
    CARGO_INCREMENTAL: '1',
    CARGO_TARGET_DIR: 'target',
    RUSTC_WRAPPER: 'other',
    SCCACHE_BASEDIRS: 'old',
  };
  const before = { ...env };
  const local = cargoEnvironment({ mode: 'local' }, env);
  const cached = cargoEnvironment({ mode: 'shared', shared }, env);
  assert.equal(local.CARGO_INCREMENTAL, '1');
  assert.equal(local.RUSTC_WRAPPER, '');
  assert.equal(cached.CARGO_INCREMENTAL, '0');
  assert.equal(cached.CARGO_TARGET_DIR, undefined);
  assert.equal(cached.SCCACHE_BASEDIRS, undefined);
  assert.equal(cached.RUSTC_WRAPPER, 'sccache');
  assert.deepEqual(env, before);
});

test('Cargo paths precede the test harness separator and preserve spaces', async (t) => {
  const { layout, root, env } = fixture(t);
  const tools = path.join(root, 'tools');
  fs.mkdirSync(tools);
  fs.copyFileSync(
    process.execPath,
    path.join(tools, process.platform === 'win32' ? 'cargo.exe' : 'cargo'),
  );
  let call;
  await invokeCargo(
    layout,
    { mode: 'local' },
    ['test', '--lib', '--', '--test-threads=1'],
    { ...env, PATH: tools },
    async (...args) => {
      call = args;
    },
  );
  const args = call[1];
  assert.equal(args.filter((arg) => arg === '--manifest-path').length, 1);
  assert(args.indexOf('--target-dir') < args.indexOf('--'));
  assert.equal(args.at(-1), '--test-threads=1');
  assert.equal(call[2].cwd, layout.worktree);
});

test('failed child compilation retains its exit code', async () => {
  await assert.rejects(
    run(process.execPath, ['-e', 'process.exit(7)'], { stdio: 'ignore' }),
    (error) => error.exitCode === 7,
  );
});

test('a worktree lock covers publication and rejects competing builds', (t) => {
  const { layout } = fixture(t);
  const release = acquireLock(layout);
  assert.throws(() => acquireLock(layout), /Another build/);
  release();
  acquireLock(layout)();
});

test('publication makes independent runnable copies and includes symbols only when requested', (t) => {
  const { root } = fixture(t);
  const source = path.join(root, 'app.exe');
  fs.writeFileSync(source, 'version one');
  fs.writeFileSync(path.join(root, 'app.pdb'), 'symbols');
  const output = path.join(root, 'normal');
  const exe = publishApplication(source, output, false);
  const debug = path.join(root, 'debug');
  publishApplication(source, debug, true);
  fs.writeFileSync(source, 'version two');
  fs.unlinkSync(source);
  assert.equal(fs.readFileSync(exe, 'utf8'), 'version one');
  assert(!fs.existsSync(path.join(output, 'app.pdb')));
  assert(fs.existsSync(path.join(debug, 'app.pdb')));
  assert.throws(() => publishApplication(source, output, false), /already exists/);
});

test('CLI cache options remain distinct from profiles and test arguments', () => {
  const request = parseArguments([
    'test',
    '--profile',
    'test-fast',
    '--lib',
    '--',
    '--test-threads=1',
    '--cache=shared',
  ]);
  assert.equal(request.cache, 'shared');
  assert.equal(request.profile, 'test-fast');
  assert.deepEqual(request.cargoArgs, [
    '--profile',
    'test-fast',
    '--lib',
    '--',
    '--test-threads=1',
  ]);
  assert.equal(parseArguments(['app']).profile, 'release');
  assert.equal(parseArguments(['app', '--debug', '--cache=local']).profile, 'debug');
  assert.throws(() => parseArguments(['test', '--manifest-path', 'elsewhere']), /owns/);
});

test('clear-cache refuses an arbitrary explicit directory', (t) => {
  const { root, worktree } = fixture(t);
  assert.throws(
    () => clearCache({ worktreeRoot: worktree, targetDir: root }),
    /dedicated directory/,
  );
  assert(fs.existsSync(worktree));
});

test('shared preflight rejects old versions, mismatched servers and unavailable statistics', (t) => {
  const { env, root } = fixture(t);
  const cache = path.join(root, 'shared');
  const scoped = { ...env, SCCACHE_DIR: cache };
  const probe = (version, location) => (_program, args) =>
    args[0] === '--version'
      ? version
      : JSON.stringify({ cache_location: 'Local disk: "' + location + '"', stats: {} });
  assert.throws(
    () => inspectSharedCache('sccache', scoped, probe('sccache 0.16.0', cache)),
    /0.17.0/,
  );
  assert.throws(
    () => inspectSharedCache('sccache', scoped, probe('sccache 0.17.0', path.join(root, 'other'))),
    /different cache/,
  );
  assert.equal(
    inspectSharedCache('sccache', scoped, probe('sccache 0.17.0', cache)).cacheDir,
    cache,
  );
  assert.throws(
    () =>
      inspectSharedCache('sccache', scoped, () => {
        throw new Error('stats unavailable');
      }),
    /stats unavailable/,
  );
});

test('statistics failure never replaces the Cargo exit code', async (t) => {
  const { layout, env } = fixture(t);
  const selection = { mode: 'shared', shared: { ...shared, program: process.execPath } };
  await assert.rejects(
    invokeCargo(layout, selection, ['check'], env, async () => {
      throw new BuildError('compiler failed', 'build_failed', 37);
    }),
    (error) => error.exitCode === 37,
  );
});

test('explicit cache clearing preserves retained applications and refuses an active build', (t) => {
  const { layout, worktree, env, root } = fixture(t);
  fs.mkdirSync(layout.target, { recursive: true });
  const source = path.join(layout.target, 'app.exe');
  fs.writeFileSync(source, 'runnable');
  const output = path.join(root, 'application');
  const exe = publishApplication(source, output, false);
  const release = acquireLock(layout);
  assert.throws(() => clearCache({ worktreeRoot: worktree }, env), /Another build/);
  release();
  clearCache({ worktreeRoot: worktree }, env);
  assert(!fs.existsSync(layout.target));
  assert.equal(fs.readFileSync(exe, 'utf8'), 'runnable');
});

test('application options reject misspelled flags instead of silently building a different mode', () => {
  assert.throws(() => parseArguments(['app', '--debgu']), /Unknown application option/);
});

test('frontend configuration is relative so Windows assets are embedded rather than treated as a URL', (t) => {
  const { layout, worktree } = fixture(t);
  const frontend = path.join(layout.ownerRoot, 'frontend', 'release');
  const configured = frontendConfigPath(worktree, frontend);
  assert(!path.isAbsolute(configured));
  assert(configured.startsWith('../'));
  assert.equal(path.resolve(worktree, 'src-tauri', configured), frontend);
});

test('review builds merge a validated build-specific Tauri identifier', (t) => {
  const { layout, worktree } = fixture(t);
  const frontend = path.join(layout.ownerRoot, 'frontend', 'release');
  const identifier =
    'dev.codex-orchestrator.review.refinement-usability.wt-worktree-one.bld-build-one';
  assert.equal(
    applicationBuildConfig(
      { applicationIdentifier: identifier, applicationLabel: 'review · worktree · build' },
      worktree,
      frontend,
    ).identifier,
    identifier,
  );
  assert.throws(
    () => applicationBuildConfig({ applicationIdentifier: 'Not Safe' }, worktree, frontend),
    /identifier is invalid/,
  );
  assert.throws(
    () => applicationBuildConfig({ applicationLabel: '\n' }, worktree, frontend),
    /label is invalid/,
  );
});

test('application receipts capture the schema compiled from the selected worktree', (t) => {
  const { worktree } = fixture(t);
  const source = path.join(worktree, 'src-tauri', 'src');
  fs.mkdirSync(source, { recursive: true });
  fs.writeFileSync(
    path.join(source, 'storage.rs'),
    'pub(crate) const ACTIVE_SCHEMA_VERSION: i64 = 55;\n',
  );
  assert.equal(applicationSchemaVersion(worktree), 55);
  fs.writeFileSync(path.join(source, 'storage.rs'), 'const SOMETHING_ELSE: i64 = 55;\n');
  assert.equal(applicationSchemaVersion(worktree), null);
});

test('debug publication copies Rust underscore-named PDBs for a hyphenated executable', (t) => {
  const { root } = fixture(t);
  const source = path.join(root, 'codex-orchestrator.exe');
  fs.writeFileSync(source, 'runnable');
  fs.writeFileSync(path.join(root, 'codex_orchestrator.pdb'), 'symbols');
  const output = path.join(root, 'published');
  publishApplication(source, output, true);
  assert.equal(fs.readFileSync(path.join(output, 'codex_orchestrator.pdb'), 'utf8'), 'symbols');
});

test('cache clearing refuses directory junctions without touching their destinations', (t) => {
  const { root, layout, worktree, env } = fixture(t);
  const foreign = path.join(root, 'foreign');
  fs.mkdirSync(foreign);
  fs.writeFileSync(path.join(foreign, 'keep'), 'keep');
  fs.mkdirSync(layout.ownerRoot, { recursive: true });
  fs.symlinkSync(foreign, layout.target, process.platform === 'win32' ? 'junction' : 'dir');
  assert.throws(() => clearCache({ worktreeRoot: worktree }, env), /linked target/);
  assert.equal(fs.readFileSync(path.join(foreign, 'keep'), 'utf8'), 'keep');
});
