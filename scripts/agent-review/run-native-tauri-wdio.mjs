import { spawn } from 'node:child_process';
import { createWriteStream } from 'node:fs';
import { mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { connect } from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import getPort, { portNumbers } from 'get-port';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const evidenceDir = path.join(root, 'test-results', 'native-tauri-wdio', 'latest');
const evidenceRoot = path.join(root, 'test-results', 'native-tauri-wdio');
if (!evidenceDir.startsWith(`${evidenceRoot}${path.sep}`)) {
  throw new Error('Refusing to clean an evidence path outside the native review directory.');
}

await rm(evidenceDir, { recursive: true, force: true });
await mkdir(evidenceDir, { recursive: true });

const embeddedPort = await getPort({ port: portNumbers(4445, 4495) });
const binaryPath = path.join(root, 'src-tauri', 'target', 'release', 'codex-orchestrator.exe');
const { environment: sanitizedEnvironment, scrubbedVariableCount } = scrubEnvironment(process.env);
const env = {
  ...sanitizedEnvironment,
  CODEX_ORCHESTRATOR_NATIVE_REVIEW_APP_DATA_DIR: path.join(evidenceDir, 'app-data'),
  NATIVE_REVIEW_BINARY_PATH: binaryPath,
  NATIVE_REVIEW_EVIDENCE_DIR: evidenceDir,
  NATIVE_REVIEW_EMBEDDED_PORT: String(embeddedPort),
  WEBVIEW2_USER_DATA_FOLDER: path.join(evidenceDir, 'webview2-profile'),
};

const startedAt = new Date().toISOString();
const skipBuild = process.argv.includes('--skip-build');
const buildExitCode = skipBuild
  ? await recordSkippedBuild()
  : await run(
      process.execPath,
      [
        path.join(root, 'node_modules', '@tauri-apps', 'cli', 'tauri.js'),
        'build',
        '--config',
        'src-tauri/tauri.native-review.conf.json',
        '--features',
        'native-review',
        '--no-bundle',
      ],
      path.join(evidenceDir, 'build.log'),
      env,
    );
const testExitCode =
  buildExitCode === 0
    ? await run(
        process.execPath,
        [
          path.join(root, 'node_modules', '@wdio', 'cli', 'bin', 'wdio.js'),
          'run',
          'wdio.native-review.conf.mjs',
        ],
        path.join(evidenceDir, 'wdio-run.log'),
        env,
      )
    : null;
const completedAt = new Date().toISOString();

const packageJson = JSON.parse(await readFile(path.join(root, 'package.json'), 'utf8'));
const logObservations = await observeForwardedLogs(path.join(evidenceDir, 'wdio-output'));
const lifecycle = {
  embeddedPortClosed: await waitForPortClosed(embeddedPort),
  appDataRemoved: await removeOwnedDirectory(env.CODEX_ORCHESTRATOR_NATIVE_REVIEW_APP_DATA_DIR),
  webview2ProfileRemoved: await removeOwnedDirectory(env.WEBVIEW2_USER_DATA_FOLDER),
  serviceSessionEnded: testExitCode !== null,
};
const lifecycleAccepted =
  lifecycle.embeddedPortClosed &&
  lifecycle.appDataRemoved &&
  lifecycle.webview2ProfileRemoved &&
  lifecycle.serviceSessionEnded;
const manifest = {
  disposition: buildExitCode === 0 && testExitCode === 0 && lifecycleAccepted ? 'passed' : 'failed',
  startedAt,
  completedAt,
  repository: {
    branch: await capture('git', ['branch', '--show-current']),
    revision: await capture('git', ['rev-parse', 'HEAD']),
    worktree: root,
    workingTreeChanges: (await capture('git', ['status', '--short']))
      .split(/\r?\n/)
      .filter(Boolean),
  },
  application: {
    mode: 'release Tauri binary with the native-review Cargo feature and alternate Tauri config',
    binaryPath,
    config: 'src-tauri/tauri.native-review.conf.json',
    appDataDir: env.CODEX_ORCHESTRATOR_NATIVE_REVIEW_APP_DATA_DIR,
    webview2Profile: env.WEBVIEW2_USER_DATA_FOLDER,
    isolatedStateRetained: false,
  },
  driver: {
    service: '@wdio/tauri-service',
    serviceVersion: packageJson.devDependencies['@wdio/tauri-service'],
    provider: 'embedded',
    webdriverPlugin: 'tauri-plugin-wdio-webdriver',
    webdriverPluginVersion: '1.2.0',
    bridgePluginVersion: packageJson.devDependencies['@wdio/tauri-plugin'],
    nativeUtilsVersion: packageJson.overrides['@wdio/native-utils'],
    embeddedPort,
    processOwnership:
      'The WDIO Tauri service spawns the application, supplies TAURI_WEBDRIVER_PORT, and terminates the owned application when the session ends.',
  },
  platform: {
    platform: process.platform,
    release: os.release(),
    arch: os.arch(),
    node: process.version,
    configuredWindow: { width: 1280, height: 820 },
    observedWindow: 'See assertions.json.',
  },
  security: {
    ambientCredentials: 'scrubbed before build and launch',
    scrubbedVariableCount,
    retainedCredentialValues: false,
  },
  scenario: {
    startingState: 'Fresh worktree-local application data and WebView2 profile.',
    actions: [
      'Launch the real Tauri release binary through the embedded WebDriver server.',
      'Observe the rendered application root.',
      'Invoke load_orchestration_native_query through browser.tauri.execute.',
      'Retain the returned contract assertion and native shell screenshot.',
    ],
    assertions: [
      'The window title is Codex Orchestrator and #root is displayed.',
      'The real Rust command returns orchestration-native-query/v2.',
      'Every persisted collection is empty in the fresh isolated database.',
      'The native shell screenshot is non-empty.',
    ],
  },
  commands: {
    build: 'npm run build:native-review-tauri',
    test: 'npx wdio run wdio.native-review.conf.mjs',
    complete: 'npm run review:native',
  },
  exitCodes: { build: buildExitCode, test: testExitCode },
  logObservations,
  lifecycle,
  producedFiles: [
    'manifest.json',
    'build.log',
    'wdio-run.log',
    'assertions.json',
    'native-shell.png',
    'wdio-output/',
  ],
  unverifiedClaims: [
    'A screenshot is retained as an observation, not standalone proof of behavior or visual fidelity.',
    'This run does not exercise a production build or production data.',
    'Command mocking was not exercised. The frontend bridge reported that defineProperty interception failed, while browser.tauri.execute and the real IPC command still passed.',
  ],
  compatibilityNotes: [
    '@wdio/tauri-service 1.2.0 declares @wdio/native-utils 2.4.0 but imports installMockSyncOverride, which first appears in 2.5.0. The package override is required for service initialization.',
  ],
};
await writeFile(
  path.join(evidenceDir, 'manifest.json'),
  `${JSON.stringify(manifest, null, 2)}\n`,
  'utf8',
);

process.exitCode = buildExitCode || testExitCode || (lifecycleAccepted ? 0 : 1);

function run(command, args, logPath, childEnv) {
  return new Promise((resolve, reject) => {
    const log = createWriteStream(logPath, { flags: 'w' });
    const child = spawn(command, args, {
      cwd: root,
      env: childEnv,
      shell: false,
      windowsHide: true,
    });
    child.stdout.on('data', (chunk) => {
      process.stdout.write(chunk);
      log.write(chunk);
    });
    child.stderr.on('data', (chunk) => {
      process.stderr.write(chunk);
      log.write(chunk);
    });
    child.on('error', (error) => {
      log.end();
      reject(error);
    });
    child.on('close', (code) => {
      log.end();
      resolve(code ?? 1);
    });
  });
}

function capture(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: root,
      shell: false,
      windowsHide: true,
    });
    let stdout = '';
    child.stdout.on('data', (chunk) => {
      stdout += chunk;
    });
    child.on('error', reject);
    child.on('close', (code) => {
      if (code === 0) {
        resolve(stdout.trim());
      } else {
        reject(new Error(`${command} exited with ${code}`));
      }
    });
  });
}

async function recordSkippedBuild() {
  await writeFile(
    path.join(evidenceDir, 'build.log'),
    'Build skipped for this rerun; using the existing native-review release binary.\n',
    'utf8',
  );
  return 0;
}

async function observeForwardedLogs(outputDir) {
  const files = await readdir(outputDir, { withFileTypes: true }).catch(() => []);
  const logFiles = files.filter((entry) => entry.isFile() && entry.name.endsWith('.log'));
  const contents = await Promise.all(
    logFiles.map(async (entry) => ({
      path: path.relative(root, path.join(outputDir, entry.name)),
      content: await readFile(path.join(outputDir, entry.name), 'utf8'),
    })),
  );
  return {
    backendForwarded: contents.some(({ content }) => content.includes('[Tauri:Backend')),
    frontendForwarded: contents.some(({ content }) => content.includes('[Tauri:Frontend')),
    commandMockingWarningObserved: contents.some(({ content }) =>
      content.includes('Invoke interception via defineProperty failed'),
    ),
    forwardingJsonWarningCount: contents.reduce(
      (count, { content }) => count + (content.match(/JSON error:/g)?.length ?? 0),
      0,
    ),
    files: contents
      .filter(
        ({ content }) => content.includes('[Tauri:Backend') || content.includes('[Tauri:Frontend'),
      )
      .map(({ path: logPath }) => logPath),
  };
}

async function waitForPortClosed(port) {
  for (let attempt = 0; attempt < 40; attempt += 1) {
    if (!(await portAcceptsConnections(port))) return true;
    await delay(250);
  }
  return false;
}

function portAcceptsConnections(port) {
  return new Promise((resolve) => {
    const socket = connect({ host: '127.0.0.1', port });
    socket.setTimeout(500);
    socket.once('connect', () => {
      socket.destroy();
      resolve(true);
    });
    const closed = () => {
      socket.destroy();
      resolve(false);
    };
    socket.once('error', closed);
    socket.once('timeout', closed);
  });
}

async function removeOwnedDirectory(directory) {
  const resolved = path.resolve(directory);
  if (!resolved.startsWith(`${evidenceDir}${path.sep}`)) {
    throw new Error(`Refusing to remove a native review directory outside ${evidenceDir}.`);
  }
  for (let attempt = 0; attempt < 20; attempt += 1) {
    await rm(resolved, { recursive: true, force: true }).catch(() => undefined);
    const stillPresent = await readdir(resolved).then(
      () => true,
      () => false,
    );
    if (!stillPresent) return true;
    await delay(250);
  }
  return false;
}

function scrubEnvironment(source) {
  const environment = {};
  let scrubbedVariableCount = 0;
  for (const [name, value] of Object.entries(source)) {
    if (
      name.toUpperCase() === 'CODEX_HOME' ||
      /token|secret|password|credential|api[_-]?key|auth/i.test(name)
    ) {
      scrubbedVariableCount += 1;
    } else if (value !== undefined) {
      environment[name] = value;
    }
  }
  return { environment, scrubbedVariableCount };
}
