import path from 'node:path';

const root = process.cwd();
const evidenceDir = process.env.NATIVE_REVIEW_EVIDENCE_DIR;
const embeddedPort = Number(process.env.NATIVE_REVIEW_EMBEDDED_PORT);
const binaryPath =
  process.env.NATIVE_REVIEW_BINARY_PATH ??
  path.join(root, 'src-tauri', 'target', 'release', 'codex-orchestrator.exe');

if (!evidenceDir || !Number.isInteger(embeddedPort)) {
  throw new Error('Run this configuration through npm run review:native.');
}

export const config = {
  runner: 'local',
  specs: ['./tests/agent-review/native-tauri-wdio.wdio.mjs'],
  maxInstances: 1,
  maxInstancesPerCapability: 1,
  services: [
    [
      '@wdio/tauri-service',
      {
        appBinaryPath: binaryPath,
        driverProvider: 'embedded',
        embeddedPort,
        captureBackendLogs: true,
        captureFrontendLogs: true,
        backendLogLevel: 'info',
        frontendLogLevel: 'info',
        logDir: path.join(evidenceDir, 'service-logs'),
        startTimeout: 90_000,
        commandTimeout: 30_000,
      },
    ],
  ],
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application: binaryPath,
      },
    },
  ],
  outputDir: path.join(evidenceDir, 'wdio-output'),
  logLevel: 'info',
  reporters: ['spec'],
  bail: 0,
  waitforTimeout: 15_000,
  connectionRetryTimeout: 90_000,
  connectionRetryCount: 1,
  framework: 'mocha',
  mochaOpts: {
    ui: 'bdd',
    timeout: 90_000,
  },
};
