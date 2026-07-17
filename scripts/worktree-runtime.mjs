import { createHash } from 'node:crypto';
import { createWriteStream } from 'node:fs';
import { appendFile, mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import http from 'node:http';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { spawn, spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

const scriptPath = fileURLToPath(import.meta.url);
const sensitiveEnvironmentPatterns = [
  /^OPENAI_API_KEY$/i,
  /^CODEX_API_KEY$/i,
  /^GH_TOKEN$/i,
  /^GITHUB_TOKEN$/i,
  /^GOOGLE_APPLICATION_CREDENTIALS$/i,
  /^AWS_(ACCESS_KEY_ID|SECRET_ACCESS_KEY|SESSION_TOKEN)$/i,
  /^AZURE_.*(TOKEN|SECRET|PASSWORD|KEY)$/i,
];

export function validateInstanceId(value) {
  if (!/^[a-zA-Z0-9][a-zA-Z0-9._-]{0,63}$/.test(value ?? '')) {
    throw new Error('instance must be 1-64 letters, numbers, dots, underscores, or hyphens');
  }
  return value;
}

export function portsForSlot(slot) {
  if (!Number.isInteger(slot) || slot < 1 || slot > 1000) {
    throw new Error('slot must be an integer from 1 through 1000');
  }
  return {
    vite: 1420 + slot * 20,
    status: 41415 + slot * 20,
  };
}

export function keyedHash(parts) {
  const hash = createHash('sha256');
  for (const part of parts) {
    const value = Buffer.isBuffer(part) ? part : Buffer.from(String(part));
    hash.update(String(value.length));
    hash.update('\0');
    hash.update(value);
    hash.update('\0');
  }
  return hash.digest('hex').slice(0, 24);
}

export function tauriOverride(manifest) {
  return {
    productName: `Codex Orchestrator [${manifest.identity.instanceId}]`,
    identifier: manifest.identity.tauriIdentifier,
    build: {
      beforeDevCommand: 'npm run dev',
      devUrl: `http://127.0.0.1:${manifest.projected.ports.vite}`,
      beforeBuildCommand: 'npm run build',
      frontendDist: manifest.projected.paths.dist,
    },
    app: {
      windows: [
        {
          title: `Codex Orchestrator [${manifest.identity.instanceId}]`,
          width: 1280,
          height: 820,
          minWidth: 960,
          minHeight: 640,
        },
      ],
    },
    bundle: {
      active: false,
    },
  };
}

export function isolatedEnvironment(source, manifest) {
  const environment = { ...source };
  const scrubbed = [];
  for (const key of Object.keys(environment)) {
    if (sensitiveEnvironmentPatterns.some((pattern) => pattern.test(key))) {
      delete environment[key];
      scrubbed.push(key);
    }
  }
  Object.assign(environment, {
    WORKTREE_RUNTIME_ROOT: manifest.projected.paths.root,
    WORKTREE_VITE_PORT: String(manifest.projected.ports.vite),
    VITE_RUNTIME_STATUS_URL: `http://127.0.0.1:${manifest.projected.ports.status}/status`,
    RUNTIME_STATUS_HOST: '127.0.0.1',
    RUNTIME_STATUS_PORT: String(manifest.projected.ports.status),
    RUNTIME_STATUS_URL: `http://127.0.0.1:${manifest.projected.ports.status}`,
    RUNTIME_STATUS_FILE: manifest.projected.paths.runtimeStatus,
    RUNTIME_INSTANCE_ID: manifest.identity.instanceId,
    RUNTIME_SESSION_ID: manifest.identity.sessionId,
    RUNTIME_WORKTREE_PATH: manifest.identity.worktreePath,
    RUNTIME_GIT_COMMIT: manifest.identity.gitCommit,
    CARGO_TARGET_DIR: manifest.projected.paths.cargoTarget,
    CODEX_HOME: manifest.projected.paths.credentials,
    CODEX_ORCHESTRATOR_APP_DATA_DIR: manifest.projected.paths.appData,
    npm_config_cache: manifest.projected.caches.node.path,
  });
  if (manifest.projected.caches.rust.mode === 'sccache') {
    environment.RUSTC_WRAPPER = 'sccache';
    environment.SCCACHE_DIR = manifest.projected.caches.rust.path;
  } else {
    delete environment.RUSTC_WRAPPER;
    delete environment.SCCACHE_DIR;
  }
  return { environment, scrubbed: scrubbed.sort() };
}

async function main() {
  const [command, ...rawArgs] = process.argv.slice(2);
  if (!command || command === 'help' || command === '--help') {
    printUsage();
    return;
  }
  const args = parseArgs(rawArgs);
  if (command === 'child') {
    await runChild(args);
    return;
  }
  if (command === 'prepare') {
    await prepare(args);
    return;
  }
  const manifest = await loadManifest(args);
  if (command === 'install') {
    await install(manifest);
  } else if (command === 'build') {
    await build(manifest);
  } else if (command === 'test') {
    await test(manifest);
  } else if (command === 'start') {
    await start(manifest);
  } else if (command === 'status') {
    console.log(JSON.stringify(await observe(manifest), null, 2));
  } else if (command === 'stop') {
    await stop(manifest, 'requested teardown');
  } else if (command === 'recover') {
    await recover(manifest);
  } else {
    throw new Error(`unknown command: ${command}`);
  }
}

function parseArgs(args) {
  const parsed = {};
  for (let index = 0; index < args.length; index += 1) {
    const value = args[index];
    if (!value.startsWith('--')) {
      throw new Error(`unexpected argument: ${value}`);
    }
    const separator = value.indexOf('=');
    if (separator > 0) {
      parsed[value.slice(2, separator)] = value.slice(separator + 1);
    } else {
      const key = value.slice(2);
      const next = args[index + 1];
      if (next && !next.startsWith('--')) {
        parsed[key] = next;
        index += 1;
      } else {
        parsed[key] = true;
      }
    }
  }
  return parsed;
}

async function prepare(args) {
  const workspace = await git('rev-parse', '--show-toplevel');
  const instanceId = validateInstanceId(String(args.instance ?? ''));
  const sessionId = validateInstanceId(String(args.session ?? instanceId));
  const slot = Number.parseInt(String(args.slot ?? ''), 10);
  const ports = portsForSlot(slot);
  const gitCommit = await git('rev-parse', 'HEAD');
  const branch = await git('branch', '--show-current');
  const gitDirectory = path.resolve(workspace, await git('rev-parse', '--git-dir'));
  const gitCommonDirectory = path.resolve(workspace, await git('rev-parse', '--git-common-dir'));
  const dirtyState = await sourceState(workspace, gitCommit);
  const runtimeRoot = path.join(workspace, '.dev', 'worktree-runtime', instanceId);
  const manifestPath = path.join(runtimeRoot, 'manifest.json');
  const repoKey = keyedHash([normalizePath(gitCommonDirectory)]);
  const sharedRoot =
    args['cache-root'] ??
    path.join(
      process.env.LOCALAPPDATA ?? os.tmpdir(),
      'Codex Orchestrator',
      'worktree-runtime-cache',
      repoKey,
    );
  const packageLock = await readFile(path.join(workspace, 'package-lock.json'));
  const cargoLock = await readFile(path.join(workspace, 'src-tauri', 'Cargo.lock'));
  const cargoManifest = await readFile(path.join(workspace, 'src-tauri', 'Cargo.toml'));
  const npmVersion = await toolVersion('npm.cmd', ['--version']);
  const rustVersion = await toolVersion('rustc', ['-vV']);
  const nodeKey = keyedHash([
    packageLock,
    process.version,
    npmVersion,
    process.platform,
    process.arch,
  ]);
  const rustKey = keyedHash([
    cargoLock,
    cargoManifest,
    rustVersion,
    process.platform,
    process.arch,
    process.env.RUSTFLAGS ?? '',
    'dev-test',
  ]);
  const hasSccache = toolAvailable('sccache', ['--version']);
  const manifest = {
    schemaVersion: 1,
    identity: {
      instanceId,
      sessionId,
      worktreePath: workspace,
      gitDirectory,
      gitCommonDirectory,
      gitCommit,
      branch: branch || null,
      dirty: dirtyState.dirty,
      sourceFingerprint: dirtyState.fingerprint,
      tauriIdentifier: `dev.codex-orchestrator.worktree.${keyedHash([
        normalizePath(workspace),
        instanceId,
      ]).slice(0, 16)}`,
    },
    projected: {
      slot,
      ports,
      paths: {
        root: runtimeRoot,
        manifest: manifestPath,
        tauriConfig: path.join(runtimeRoot, 'tauri.worktree.conf.json'),
        dist: path.join(runtimeRoot, 'dist'),
        viteCache: path.join(runtimeRoot, 'vite-cache'),
        cargoTarget: path.join(runtimeRoot, 'cargo-target'),
        appData: path.join(runtimeRoot, 'app-data'),
        credentials: path.join(runtimeRoot, 'credentials', 'codex-home'),
        logs: path.join(runtimeRoot, 'logs'),
        screenshots: path.join(runtimeRoot, 'screenshots'),
        recordings: path.join(runtimeRoot, 'recordings'),
        runtimeStatus: path.join(runtimeRoot, 'runtime-status.json'),
      },
      caches: {
        node: {
          key: nodeKey,
          path: path.join(sharedRoot, 'npm', nodeKey),
          invalidation: ['package-lock.json', 'node version', 'npm version', 'OS', 'architecture'],
        },
        rust: {
          key: rustKey,
          path: path.join(sharedRoot, 'sccache', rustKey),
          mode: hasSccache ? 'sccache' : 'isolated-target-only',
          invalidation: [
            'Cargo.lock',
            'Cargo.toml',
            'rustc verbose version',
            'RUSTFLAGS',
            'profile',
            'OS',
            'architecture',
          ],
        },
      },
      commands: {
        install: 'npm ci',
        build: 'npm run build:tauri -- --debug --no-bundle --config <instance-config>',
        test: [
          'npm run test:worktree-runtime',
          'npm test -- src/app/App.test.tsx',
          'cargo test --manifest-path src-tauri/Cargo.toml runtime::',
        ],
        launch: 'npm run dev:tauri -- --config <instance-config>',
      },
    },
    observed: {
      preparedAt: new Date().toISOString(),
      dependencyInstall: null,
      build: null,
      tests: null,
      launch: null,
      processes: {},
      recovery: null,
    },
  };
  await ensureProjectedDirectories(manifest);
  await writeJson(manifest.projected.paths.tauriConfig, tauriOverride(manifest));
  await writeManifest(manifest);
  console.log(manifest.projected.paths.manifest);
}

async function install(manifest) {
  await ensurePreparedSource(manifest);
  await mkdir(manifest.projected.caches.node.path, { recursive: true });
  const startedAt = new Date().toISOString();
  await runLogged(manifest, 'install', 'npm.cmd', ['ci'], process.env);
  manifest.observed.dependencyInstall = {
    startedAt,
    completedAt: new Date().toISOString(),
    nodeCacheKey: manifest.projected.caches.node.key,
  };
  await writeManifest(manifest);
}

async function build(manifest) {
  await ensurePreparedSource(manifest);
  await ensureDependencies(manifest);
  const startedAt = new Date().toISOString();
  await runLogged(
    manifest,
    'build',
    'npm.cmd',
    [
      'run',
      'build:tauri',
      '--',
      '--debug',
      '--no-bundle',
      '--config',
      manifest.projected.paths.tauriConfig,
    ],
    process.env,
    true,
  );
  await writeBuildIdentity(manifest);
  manifest.observed.build = {
    startedAt,
    completedAt: new Date().toISOString(),
    sourceFingerprint: manifest.identity.sourceFingerprint,
    cargoTarget: manifest.projected.paths.cargoTarget,
    dist: manifest.projected.paths.dist,
  };
  await writeManifest(manifest);
}

async function test(manifest) {
  await ensurePreparedSource(manifest);
  await ensureDependencies(manifest);
  const startedAt = new Date().toISOString();
  await runLogged(
    manifest,
    'test-worktree-runtime',
    'npm.cmd',
    ['run', 'test:worktree-runtime'],
    process.env,
  );
  await runLogged(
    manifest,
    'test-runtime-status',
    'npm.cmd',
    ['test', '--', 'src/app/App.test.tsx'],
    process.env,
  );
  await runLogged(
    manifest,
    'test-rust',
    'cargo',
    ['test', '--manifest-path', 'src-tauri/Cargo.toml', 'runtime::'],
    process.env,
    true,
  );
  manifest.observed.tests = {
    startedAt,
    completedAt: new Date().toISOString(),
    sourceFingerprint: manifest.identity.sourceFingerprint,
    scope: 'worktree runtime, status client, and Rust runtime/process boundaries',
  };
  await writeManifest(manifest);
}

async function start(manifest) {
  await ensurePreparedSource(manifest);
  await ensureDependencies(manifest);
  const before = await observe(manifest);
  if (before.processes.some((process) => process.alive)) {
    throw new Error('instance already has a live owned process; inspect status or stop it first');
  }
  if (before.stale) {
    throw new Error('instance has stale launch state; run recover before starting it again');
  }
  await requireFreePort(manifest.projected.ports.status);
  await requireFreePort(manifest.projected.ports.vite);
  const { environment, scrubbed } = isolatedEnvironment(process.env, manifest);
  const processes = {};
  for (const role of ['status', 'app']) {
    const child = spawn(
      process.execPath,
      [scriptPath, 'child', '--manifest', manifest.projected.paths.manifest, '--role', role],
      {
        cwd: manifest.identity.worktreePath,
        detached: true,
        stdio: 'ignore',
        env: environment,
        windowsHide: true,
      },
    );
    child.unref();
    processes[role] = { pid: child.pid, startedAt: new Date().toISOString() };
  }
  manifest.observed.processes = processes;
  manifest.observed.launch = {
    requestedAt: new Date().toISOString(),
    credentialEnvironmentScrubbed: scrubbed,
    healthObservedAt: null,
  };
  await writeManifest(manifest);
  try {
    await waitForHealthy(manifest);
    const health = await observe(manifest);
    manifest.observed.launch.healthObservedAt = new Date().toISOString();
    manifest.observed.launch.applicationProcessObserved = health.applicationProcessObserved;
    await writeManifest(manifest);
    console.log(JSON.stringify(health, null, 2));
  } catch (error) {
    await stop(manifest, `launch failed: ${error.message}`);
    throw error;
  }
}

async function runChild(args) {
  const manifest = await readJson(requiredPath(args.manifest));
  const role = args.role;
  if (role !== 'status' && role !== 'app') {
    throw new Error('child role must be status or app');
  }
  const logPath = path.join(manifest.projected.paths.logs, `${role}.log`);
  await mkdir(path.dirname(logPath), { recursive: true });
  const logHandle = await import('node:fs').then(({ openSync }) => openSync(logPath, 'a'));
  const { environment } = isolatedEnvironment(process.env, manifest);
  const spec =
    role === 'status'
      ? {
          program: process.execPath,
          args: [
            path.join(manifest.identity.worktreePath, 'scripts', 'runtime-status-server.mjs'),
            '--runtime-instance',
            manifest.identity.instanceId,
          ],
        }
      : {
          program: 'npm.cmd',
          args: ['run', 'dev:tauri', '--', '--config', manifest.projected.paths.tauriConfig],
          needsVisualStudio: true,
        };
  const command = platformCommand(spec.program, spec.args, spec.needsVisualStudio);
  const child = spawn(command.program, command.args, {
    cwd: manifest.identity.worktreePath,
    env: environment,
    stdio: ['ignore', logHandle, logHandle],
    windowsHide: true,
    windowsVerbatimArguments: command.program.toLowerCase().endsWith('cmd.exe'),
  });
  const exit = await new Promise((resolve, reject) => {
    child.once('error', reject);
    child.once('exit', (code, signal) => resolve({ code, signal }));
  });
  await writeJson(path.join(manifest.projected.paths.root, `${role}.exit.json`), {
    role,
    exitedAt: new Date().toISOString(),
    ...exit,
  });
  process.exitCode = exit.code ?? 1;
}

async function observe(manifest) {
  const roots = [];
  for (const [role, processRecord] of Object.entries(manifest.observed.processes ?? {})) {
    const ownership = await inspectOwnedProcess(
      processRecord.pid,
      manifest.projected.paths.manifest,
      role,
    );
    roots.push({ role, pid: processRecord.pid, ...ownership });
  }
  const tree = process.platform === 'win32' ? windowsProcessTree(roots) : [];
  const statusHealth = await requestJson(
    `http://127.0.0.1:${manifest.projected.ports.status}/health`,
  );
  const viteHealth = await requestStatus(`http://127.0.0.1:${manifest.projected.ports.vite}/`);
  const applicationProcessObserved = tree.some(
    (process) =>
      process.rootRole === 'app' && process.name.toLowerCase() === 'codex-orchestrator.exe',
  );
  const expectedRunning = roots.length > 0;
  const stale =
    expectedRunning &&
    (roots.some((process) => !process.alive || !process.owned) ||
      !statusHealth.ok ||
      statusHealth.body?.owner?.instanceId !== manifest.identity.instanceId ||
      !viteHealth.ok);
  return {
    identity: manifest.identity,
    projected: {
      ports: manifest.projected.ports,
      paths: manifest.projected.paths,
    },
    observed: {
      preparedAt: manifest.observed.preparedAt,
      dependencyInstall: manifest.observed.dependencyInstall,
      build: manifest.observed.build,
      tests: manifest.observed.tests,
      launch: manifest.observed.launch,
    },
    processes: roots,
    processTree: tree,
    health: {
      status: statusHealth,
      vite: viteHealth,
    },
    applicationProcessObserved,
    stale,
  };
}

async function stop(manifest, reason) {
  const observation = await observe(manifest);
  const refusals = observation.processes.filter((process) => process.alive && !process.owned);
  if (refusals.length > 0) {
    throw new Error(
      `refusing teardown because ownership was not proven for PIDs ${refusals
        .map((process) => process.pid)
        .join(', ')}`,
    );
  }
  for (const processRecord of observation.processes.filter(
    (process) => process.alive && process.owned,
  )) {
    terminateOwnedTree(processRecord.pid);
  }
  manifest.observed.processes = {};
  manifest.observed.launch = manifest.observed.launch
    ? {
        ...manifest.observed.launch,
        stoppedAt: new Date().toISOString(),
        stopReason: reason,
      }
    : null;
  await writeManifest(manifest);
  console.log(`Stopped ${manifest.identity.instanceId}`);
}

async function recover(manifest) {
  const observation = await observe(manifest);
  if (!observation.stale) {
    console.log(`Instance ${manifest.identity.instanceId} is not stale`);
    return;
  }
  await stop(manifest, 'stale-instance recovery');
  for (const role of ['status', 'app']) {
    await rm(path.join(manifest.projected.paths.root, `${role}.exit.json`), { force: true });
  }
  manifest.observed.recovery = {
    recoveredAt: new Date().toISOString(),
    priorObservation: {
      processes: observation.processes,
      health: observation.health,
    },
  };
  await writeManifest(manifest);
  console.log(`Recovered stale instance ${manifest.identity.instanceId}`);
}

async function waitForHealthy(manifest) {
  const deadline = Date.now() + 240_000;
  while (Date.now() < deadline) {
    const observation = await observe(manifest);
    const rootsHealthy =
      observation.processes.length === 2 &&
      observation.processes.every((process) => process.alive && process.owned);
    if (
      rootsHealthy &&
      observation.health.status.ok &&
      observation.health.status.body?.owner?.instanceId === manifest.identity.instanceId &&
      observation.health.vite.ok &&
      observation.applicationProcessObserved
    ) {
      return;
    }
    if (observation.processes.some((process) => !process.alive)) {
      throw new Error('an owned launch process exited before health was established');
    }
    await delay(500);
  }
  throw new Error('timed out waiting for status, Vite, and the Tauri application process');
}

async function ensurePreparedSource(manifest) {
  const currentCommit = await git('rev-parse', 'HEAD');
  const current = await sourceState(manifest.identity.worktreePath, currentCommit);
  if (
    currentCommit !== manifest.identity.gitCommit ||
    current.fingerprint !== manifest.identity.sourceFingerprint
  ) {
    throw new Error(
      'source changed since prepare; rerun prepare to refresh identity and cache keys',
    );
  }
}

async function ensureDependencies(manifest) {
  try {
    await stat(path.join(manifest.identity.worktreePath, 'node_modules'));
  } catch {
    throw new Error('node_modules is missing; run install for this instance first');
  }
}

async function ensureProjectedDirectories(manifest) {
  const paths = manifest.projected.paths;
  await Promise.all(
    [
      paths.root,
      paths.cargoTarget,
      paths.appData,
      paths.credentials,
      paths.logs,
      paths.screenshots,
      paths.recordings,
    ].map((directory) => mkdir(directory, { recursive: true })),
  );
}

async function writeBuildIdentity(manifest) {
  await mkdir(manifest.projected.paths.dist, { recursive: true });
  await writeJson(path.join(manifest.projected.paths.dist, 'runtime-identity.json'), {
    instanceId: manifest.identity.instanceId,
    sessionId: manifest.identity.sessionId,
    worktreePath: manifest.identity.worktreePath,
    gitCommit: manifest.identity.gitCommit,
    sourceFingerprint: manifest.identity.sourceFingerprint,
    builtAt: new Date().toISOString(),
  });
}

async function loadManifest(args) {
  if (args.manifest) {
    return readJson(requiredPath(args.manifest));
  }
  const instance = validateInstanceId(String(args.instance ?? ''));
  const workspace = await git('rev-parse', '--show-toplevel');
  return readJson(path.join(workspace, '.dev', 'worktree-runtime', instance, 'manifest.json'));
}

async function writeManifest(manifest) {
  await writeJson(manifest.projected.paths.manifest, manifest);
}

async function writeJson(filePath, value) {
  await mkdir(path.dirname(filePath), { recursive: true });
  const temporary = `${filePath}.${process.pid}.tmp`;
  await writeFile(temporary, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
  await rm(filePath, { force: true });
  await import('node:fs/promises').then(({ rename }) => rename(temporary, filePath));
}

async function readJson(filePath) {
  return JSON.parse(await readFile(filePath, 'utf8'));
}

async function sourceState(workspace, commit) {
  const status = spawnSync('git', ['-C', workspace, 'status', '--porcelain=v1', '-z'], {
    encoding: null,
  });
  if (status.status !== 0) {
    throw new Error(status.stderr?.toString('utf8').trim() || 'git status failed');
  }
  const diff = spawnSync('git', ['-C', workspace, 'diff', '--binary', 'HEAD'], {
    encoding: null,
    maxBuffer: 64 * 1024 * 1024,
  });
  if (diff.status !== 0) {
    throw new Error(diff.stderr?.toString('utf8').trim() || 'git diff failed');
  }
  const untracked = status.stdout
    .toString('utf8')
    .split('\0')
    .filter((entry) => entry.startsWith('?? '))
    .map((entry) => entry.slice(3))
    .sort();
  const untrackedParts = [];
  for (const relative of untracked) {
    const absolute = path.join(workspace, relative);
    try {
      untrackedParts.push(relative, await readFile(absolute));
    } catch {
      untrackedParts.push(relative);
    }
  }
  return {
    dirty: status.stdout.length > 0,
    fingerprint: keyedHash([commit, status.stdout, diff.stdout, ...untrackedParts]),
  };
}

async function git(...args) {
  const result = spawnSync('git', args, { encoding: 'utf8' });
  if (result.status !== 0) {
    throw new Error(result.stderr.trim() || `git ${args.join(' ')} failed`);
  }
  return result.stdout.trim();
}

async function toolVersion(program, args) {
  const command = platformCommand(program, args);
  const result = spawnSync(command.program, command.args, {
    encoding: 'utf8',
    windowsVerbatimArguments: command.program.toLowerCase().endsWith('cmd.exe'),
  });
  if (result.status !== 0) {
    throw new Error(`${program} is unavailable`);
  }
  return result.stdout.trim();
}

function toolAvailable(program, args) {
  const command = platformCommand(program, args);
  return (
    spawnSync(command.program, command.args, {
      stdio: 'ignore',
      windowsVerbatimArguments: command.program.toLowerCase().endsWith('cmd.exe'),
    }).status === 0
  );
}

async function runLogged(
  manifest,
  name,
  program,
  args,
  sourceEnvironment,
  needsVisualStudio = false,
) {
  const { environment } = isolatedEnvironment(sourceEnvironment, manifest);
  const command = platformCommand(program, args, needsVisualStudio);
  const logPath = path.join(manifest.projected.paths.logs, `${name}.log`);
  await appendFile(
    logPath,
    `\n[${new Date().toISOString()}] ${program} ${args.join(' ')}\n`,
    'utf8',
  );
  await new Promise((resolve, reject) => {
    const log = createWriteStream(logPath, { flags: 'a' });
    const child = spawn(command.program, command.args, {
      cwd: manifest.identity.worktreePath,
      env: environment,
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
      windowsVerbatimArguments: command.program.toLowerCase().endsWith('cmd.exe'),
    });
    for (const stream of [child.stdout, child.stderr]) {
      stream.on('data', (chunk) => {
        process.stdout.write(chunk);
        log.write(chunk);
      });
    }
    child.once('error', (error) => {
      log.end();
      reject(error);
    });
    child.once('exit', (code, signal) => {
      log.end(() => {
        if (code === 0) {
          resolve();
        } else {
          reject(new Error(`${name} exited with ${signal ? `signal ${signal}` : `code ${code}`}`));
        }
      });
    });
  });
}

function platformCommand(program, args, needsVisualStudio = false) {
  const requiresCommandInterpreter = /\.(bat|cmd)$/i.test(program);
  if (process.platform !== 'win32' || (!needsVisualStudio && !requiresCommandInterpreter)) {
    return { program, args };
  }
  const vcvars =
    process.env.VCVARS64 ??
    'C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\BuildTools\\VC\\Auxiliary\\Build\\vcvars64.bat';
  const command = [program, ...args].map(quoteCmdArgument).join(' ');
  const prefix = needsVisualStudio ? `call ${quoteCmdArgument(vcvars)} >nul && ` : '';
  return {
    program: 'cmd.exe',
    args: ['/d', '/c', `${prefix}${command}`],
  };
}

function quoteCmdArgument(value) {
  const text = String(value);
  return /^[a-zA-Z0-9_./:=+-]+$/.test(text) ? text : `"${text.replaceAll('"', '""')}"`;
}

async function inspectOwnedProcess(pid, manifestPath, role) {
  if (!Number.isInteger(pid)) {
    return { alive: false, owned: false };
  }
  if (process.platform !== 'win32') {
    try {
      process.kill(pid, 0);
      return {
        alive: true,
        owned: false,
        reason: 'ownership proof is implemented only on Windows',
      };
    } catch {
      return { alive: false, owned: false };
    }
  }
  const command = [
    `$p = Get-CimInstance Win32_Process -Filter "ProcessId = ${pid}"`,
    'if ($null -eq $p) { exit 3 }',
    '$p | Select-Object ProcessId,ParentProcessId,Name,CommandLine,CreationDate | ConvertTo-Json -Compress',
  ].join('; ');
  const result = spawnSync(
    'powershell.exe',
    ['-NoProfile', '-NonInteractive', '-Command', command],
    {
      encoding: 'utf8',
    },
  );
  if (result.status === 3 || !result.stdout.trim()) {
    return { alive: false, owned: false };
  }
  if (result.status !== 0) {
    return { alive: true, owned: false, reason: result.stderr.trim() || 'process query failed' };
  }
  const processInfo = JSON.parse(result.stdout);
  const commandLine = String(processInfo.CommandLine ?? '')
    .replaceAll('\\', '/')
    .toLowerCase();
  const owned =
    commandLine.includes('worktree-runtime.mjs') &&
    commandLine.includes(' child ') &&
    commandLine.includes(normalizePath(manifestPath).toLowerCase()) &&
    commandLine.includes(`--role ${role}`);
  return {
    alive: true,
    owned,
    name: processInfo.Name,
    startedAt: processInfo.CreationDate,
    ...(owned ? {} : { reason: 'command line did not match the instance manifest and role' }),
  };
}

function windowsProcessTree(roots) {
  const liveRoots = roots.filter((root) => root.alive && root.owned);
  if (liveRoots.length === 0) {
    return [];
  }
  const command =
    'Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,Name | ConvertTo-Json -Compress';
  const result = spawnSync(
    'powershell.exe',
    ['-NoProfile', '-NonInteractive', '-Command', command],
    {
      encoding: 'utf8',
      maxBuffer: 16 * 1024 * 1024,
    },
  );
  if (result.status !== 0 || !result.stdout.trim()) {
    return [];
  }
  const all = [JSON.parse(result.stdout)].flat();
  const byParent = new Map();
  for (const processInfo of all) {
    const parent = Number(processInfo.ParentProcessId);
    const children = byParent.get(parent) ?? [];
    children.push(processInfo);
    byParent.set(parent, children);
  }
  const output = [];
  for (const root of liveRoots) {
    const queue = [...(byParent.get(root.pid) ?? [])];
    while (queue.length > 0) {
      const current = queue.shift();
      output.push({
        rootRole: root.role,
        pid: Number(current.ProcessId),
        parentPid: Number(current.ParentProcessId),
        name: String(current.Name),
      });
      queue.push(...(byParent.get(Number(current.ProcessId)) ?? []));
    }
  }
  return output;
}

function terminateOwnedTree(pid) {
  if (process.platform !== 'win32') {
    throw new Error('tree teardown is implemented only on Windows');
  }
  const result = spawnSync('taskkill.exe', ['/PID', String(pid), '/T', '/F'], {
    encoding: 'utf8',
  });
  if (result.status !== 0 && !/not found/i.test(result.stderr)) {
    throw new Error(result.stderr.trim() || `taskkill failed for PID ${pid}`);
  }
}

async function requestJson(url) {
  const response = await httpRequest(url);
  if (!response.ok) {
    return response;
  }
  try {
    return { ...response, body: JSON.parse(response.body) };
  } catch {
    return { ...response, ok: false, error: 'invalid JSON response' };
  }
}

async function requestStatus(url) {
  return httpRequest(url);
}

function httpRequest(url) {
  return new Promise((resolve) => {
    const request = http.get(url, { timeout: 1000 }, (response) => {
      const chunks = [];
      response.on('data', (chunk) => chunks.push(chunk));
      response.on('end', () =>
        resolve({
          ok: response.statusCode >= 200 && response.statusCode < 400,
          status: response.statusCode,
          body: Buffer.concat(chunks).toString('utf8'),
        }),
      );
    });
    request.on('timeout', () => request.destroy(new Error('timeout')));
    request.on('error', (error) => resolve({ ok: false, error: error.message }));
  });
}

async function requireFreePort(port) {
  const available = await new Promise((resolve) => {
    const server = net.createServer();
    server.once('error', () => resolve(false));
    server.listen(port, '127.0.0.1', () => server.close(() => resolve(true)));
  });
  if (!available) {
    throw new Error(`port ${port} is already in use`);
  }
}

function requiredPath(value) {
  if (typeof value !== 'string' || !path.isAbsolute(value)) {
    throw new Error('manifest must be an absolute path');
  }
  return value;
}

function normalizePath(value) {
  return path.resolve(value).replaceAll('\\', '/');
}

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function printUsage() {
  console.log(`Worktree runtime proof

  npm run runtime:worktree -- prepare --instance <id> --session <id> --slot <1-1000>
  npm run runtime:worktree -- install|build|test|start|status|stop|recover --instance <id>

Each instance writes an ownership manifest under .dev/worktree-runtime/<id>.`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
