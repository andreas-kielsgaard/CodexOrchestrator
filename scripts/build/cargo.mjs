import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { createHash } from 'node:crypto';
import { BuildError, canonical, capture, contained, executable, run } from './process.mjs';

export const cacheModes = ['auto', 'local', 'shared'];

export function hasArtifacts(target, profile) {
  try {
    return fs.readdirSync(path.join(target, profile, 'deps')).length > 0;
  } catch {
    return false;
  }
}

export function cacheLayout(worktree, targetDir, env = process.env) {
  const root = canonical(worktree);
  const identity = process.platform === 'win32' ? root.toLowerCase() : root;
  const key = createHash('sha256').update(identity).digest('hex').slice(0, 16);
  const storage = path.resolve(
    env.ORCHESTRATOR_BUILD_CACHE_ROOT ??
      path.join(
        env.LOCALAPPDATA ?? path.join(os.homedir(), '.cache'),
        'CodexOrchestrator',
        'build-cache',
      ),
  );
  const ownerRoot = path.join(storage, key);
  const ordinary = path.join(root, 'src-tauri', 'target');
  // Prefer a target already used by this tool even after another profile is built.
  let recorded;
  try {
    recorded = JSON.parse(fs.readFileSync(path.join(ownerRoot, 'owner.json'), 'utf8'));
  } catch {
    /* first use */
  }
  const populated = ['debug', 'release', 'test-fast'].some((profile) =>
    hasArtifacts(ordinary, profile),
  );
  const target = path.resolve(
    targetDir ??
      env.CARGO_TARGET_DIR ??
      (recorded?.worktree === root ? recorded.target : null) ??
      (populated ? ordinary : path.join(ownerRoot, 't')),
  );
  if (target === root || contained(target, root) || target === storage || target === ownerRoot) {
    throw new BuildError(
      'The compiler target must be a dedicated directory, not the worktree or storage root.',
      'invalid_request',
    );
  }
  return { worktree: root, key, ownerRoot, storage, target };
}

export function acquireLock(layout) {
  fs.mkdirSync(layout.ownerRoot, { recursive: true });
  const lock = path.join(layout.ownerRoot, 'build.lock');
  let fd;
  try {
    fd = fs.openSync(lock, 'wx');
  } catch (error) {
    if (error.code !== 'EEXIST') throw error;
    let pid;
    try {
      pid = JSON.parse(fs.readFileSync(lock, 'utf8')).pid;
    } catch {
      /* an owner may be writing */
    }
    if (Number.isInteger(pid)) {
      try {
        process.kill(pid, 0);
      } catch (processError) {
        if (processError.code === 'ESRCH') {
          fs.unlinkSync(lock);
          return acquireLock(layout);
        }
      }
    }
    throw new BuildError(
      'Another build is using this worktree. Wait for it to finish.',
      'output_unavailable',
    );
  }
  try {
    fs.writeFileSync(fd, JSON.stringify({ pid: process.pid, worktree: layout.worktree }));
    fs.writeFileSync(
      path.join(layout.ownerRoot, 'owner.json'),
      JSON.stringify({ worktree: layout.worktree, target: layout.target }, null, 2),
    );
  } catch (error) {
    fs.closeSync(fd);
    fs.unlinkSync(lock);
    throw error;
  }
  return () => {
    fs.closeSync(fd);
    fs.unlinkSync(lock);
  };
}

function findSccache(env) {
  try {
    return executable('sccache', env);
  } catch (error) {
    if (process.platform !== 'win32') throw error;
    // A long-running agent may have inherited PATH before sccache was installed.
    const persisted = capture('powershell.exe', [
      '-NoProfile',
      '-Command',
      "[Environment]::GetEnvironmentVariable('Path','User') + ';' + [Environment]::GetEnvironmentVariable('Path','Machine')",
    ]);
    return executable('sccache', { PATH: persisted });
  }
}

export function sharedPreflight(env = process.env) {
  return inspectSharedCache(findSccache(env), env);
}

export function inspectSharedCache(program, env = process.env, read = capture) {
  const version = read(program, ['--version']);
  const parsed = /^sccache (\d+)\.(\d+)\.(\d+)/.exec(version);
  if (!parsed || (Number(parsed[1]) === 0 && Number(parsed[2]) < 17)) {
    throw new BuildError(
      'Shared caching requires sccache 0.17.0 or newer.',
      'toolchain_unavailable',
    );
  }
  const cacheDir = path.resolve(
    env.SCCACHE_DIR ??
      path.join(
        env.LOCALAPPDATA ?? path.join(os.homedir(), '.cache'),
        'Mozilla',
        'sccache',
        'cache',
      ),
  );
  const scopedEnv = { ...env, SCCACHE_DIR: cacheDir, SCCACHE_CLIENT_SIDE: '1' };
  const before = JSON.parse(
    read(program, ['--show-stats', '--stats-format', 'json'], { env: scopedEnv }),
  );
  const active = /^Local disk: "(.+)"$/.exec(before.cache_location ?? '');
  if (!active || path.resolve(active[1]).toLowerCase() !== cacheDir.toLowerCase()) {
    throw new BuildError(
      'The running sccache server uses a different cache directory. Finish cached builds before restarting it.',
      'toolchain_unavailable',
    );
  }
  return { program, cacheDir, before };
}

export function selectCache(
  layout,
  profile,
  requested = 'auto',
  env = process.env,
  preflight = sharedPreflight,
) {
  if (!cacheModes.includes(requested))
    throw new BuildError(`Unknown cache mode: ${requested}`, 'invalid_request');
  if (requested === 'local' || (requested === 'auto' && hasArtifacts(layout.target, profile))) {
    return {
      mode: 'local',
      reason: requested === 'local' ? 'explicit choice' : `local ${profile} artifacts exist`,
    };
  }
  try {
    return {
      mode: 'shared',
      reason: requested === 'shared' ? 'explicit choice' : `no local ${profile} artifacts`,
      shared: preflight(env),
    };
  } catch (error) {
    if (requested === 'shared') throw error;
    return { mode: 'local', reason: `shared cache unavailable: ${error.message}` };
  }
}

export function cargoEnvironment(selection, env = process.env) {
  const result = { ...env };
  delete result.RUSTC_WRAPPER;
  delete result.RUSTC_WORKSPACE_WRAPPER;
  if (selection.mode === 'shared') {
    delete result.CARGO_TARGET_DIR;
    delete result.SCCACHE_BASEDIRS;
    result.CARGO_INCREMENTAL = '0';
    result.RUSTC_WRAPPER = selection.shared.program;
    result.SCCACHE_DIR = selection.shared.cacheDir;
    result.SCCACHE_CLIENT_SIDE = '1';
  }
  // An empty wrapper overrides a global Cargo config in local mode too.
  else result.RUSTC_WRAPPER = '';
  return result;
}

export function logSelection(selection, layout, profile) {
  console.log(`Compiler cache: ${selection.mode} (${selection.reason})`);
  console.log(`Cargo target: ${layout.target} | profile: ${profile}`);
  if (selection.shared)
    console.log(
      `Shared compiler cache: ${selection.shared.cacheDir}; incremental compilation disabled for this invocation`,
    );
}

function countMap(value) {
  return Object.values(value ?? {}).reduce((total, n) => total + Number(n), 0);
}
export function reportStatistics(selection, env) {
  if (!selection.shared) return;
  try {
    const after = JSON.parse(
      capture(selection.shared.program, ['--show-stats', '--stats-format', 'json'], { env }),
    );
    const before = selection.shared.before.stats;
    const hits = countMap(after.stats.cache_hits?.counts) - countMap(before.cache_hits?.counts);
    const misses =
      countMap(after.stats.cache_misses?.counts) - countMap(before.cache_misses?.counts);
    console.log(
      hits < 0 || misses < 0
        ? 'Shared cache statistics restarted during compilation.'
        : `Shared cache activity during command window: ${hits} hits, ${misses} misses (may include other builds).`,
    );
  } catch (error) {
    console.warn(`Shared cache statistics unavailable: ${error.message}`);
  }
}

export async function invokeCargo(layout, selection, args, env = process.env, runner = run) {
  const scopedEnv = cargoEnvironment(selection, env);
  const cwd =
    selection.mode === 'shared'
      ? path.join(
          env.LOCALAPPDATA ?? path.join(os.homedir(), '.cache'),
          'CodexOrchestrator',
          'cargo-sccache-cwd',
        )
      : layout.worktree;
  fs.mkdirSync(cwd, { recursive: true });
  const cargoArgs = [...args];
  const separator = cargoArgs.indexOf('--');
  const harness = separator < 0 ? [] : cargoArgs.splice(separator);
  try {
    await runner(
      executable('cargo', env),
      [
        ...cargoArgs,
        '--manifest-path',
        path.join(layout.worktree, 'src-tauri', 'Cargo.toml'),
        '--target-dir',
        layout.target,
        ...harness,
      ],
      { cwd, env: scopedEnv },
    );
  } finally {
    reportStatistics(selection, scopedEnv);
  }
}

export function profileFromArgs(args) {
  const index = args.indexOf('--profile');
  const profile =
    index >= 0
      ? args[index + 1]
      : args.find((item) => item.startsWith('--profile='))?.split('=')[1];
  return profile ?? (args.includes('--release') ? 'release' : 'debug');
}

export async function cargoCommand(request) {
  const layout = cacheLayout(request.worktreeRoot, request.targetDir);
  const release = acquireLock(layout);
  try {
    const selection = selectCache(layout, request.profile, request.cache);
    logSelection(selection, layout, request.profile);
    const args = [request.action, ...request.cargoArgs];
    const env =
      request.testDebug === undefined
        ? process.env
        : { ...process.env, CARGO_PROFILE_TEST_DEBUG: request.testDebug };
    await invokeCargo(layout, selection, args, env);
  } finally {
    release();
  }
}

export function clearCache(request, env = process.env) {
  const layout = cacheLayout(request.worktreeRoot, request.targetDir, env);
  const allowed = path.join(layout.worktree, 'src-tauri', 'target');
  if (layout.target !== allowed && !contained(layout.ownerRoot, layout.target))
    throw new BuildError(
      "Cache clearing only removes this worktree's standard target or its tool-owned target.",
      'invalid_request',
    );
  const release = acquireLock(layout);
  try {
    if (fs.existsSync(layout.target) && canonical(layout.target) !== layout.target)
      throw new BuildError('Refusing to clear a linked target.', 'invalid_request');
    fs.rmSync(layout.target, { recursive: true, force: true });
    console.log(`Cleared compiler cache for ${layout.worktree}: ${layout.target}`);
  } finally {
    release();
  }
}
