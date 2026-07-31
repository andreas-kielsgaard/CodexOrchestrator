import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { access, mkdir, mkdtemp, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const cliPath = fileURLToPath(new globalThis.URL('../review-app.mjs', import.meta.url));

test('detached launch rejects launcher/evidence collisions before creating a child artifact', async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'review-app-launch-paths-'));
  t.after(() => rm(root, { recursive: true, force: true }));

  const cases = [
    { launcher: 'watcher-log', evidence: 'before' },
    { launcher: 'watcher-log', evidence: 'before-out' },
    { launcher: 'watcher-log', evidence: 'after-out' },
    { launcher: 'launch-out', evidence: 'comparison-out' },
    { launcher: 'launch-out', evidence: 'human-out' },
  ];

  for (const value of cases) {
    await t.test(`${value.launcher} cannot reuse ${value.evidence}`, async () => {
      const caseRoot = path.join(root, `${value.launcher}-${value.evidence}`);
      await mkdir(caseRoot, { recursive: true });
      const collisionPath = path.join(caseRoot, 'collision.out');
      const watcherLog =
        value.launcher === 'watcher-log' ? collisionPath : path.join(caseRoot, 'watcher.log');
      const launchOut =
        value.launcher === 'launch-out' ? collisionPath : path.join(caseRoot, 'launch.json');
      const evidenceArguments = [`--${value.evidence}`, collisionPath];
      await assert.rejects(
        execFileAsync(
          process.execPath,
          [
            cliPath,
            'launch-wait',
            '--workspace',
            caseRoot,
            '--exe',
            path.join(caseRoot, 'app.exe'),
            '--instance',
            'path-test',
            '--app-data-dir',
            path.join(caseRoot, 'app-data'),
            '--evidence-root',
            caseRoot,
            '--condition',
            'durable',
            '--timeout-ms',
            '5000',
            '--out',
            path.join(caseRoot, 'wait-result.json'),
            '--watcher-log',
            watcherLog,
            '--launch-out',
            launchOut,
            ...evidenceArguments,
          ],
          { windowsHide: true },
        ),
        /must not reuse/u,
      );

      assert.equal(await exists(watcherLog), false);
      assert.equal(await exists(launchOut), false);
      assert.equal(await exists(path.join(caseRoot, 'wait-result.json')), false);
    });
  }
});

test('callback option is rejected before any detached artifact is created', async (t) => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'review-app-callback-disabled-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const watcherLog = path.join(root, 'watcher.log');
  const launchOut = path.join(root, 'launch.json');
  const waitResult = path.join(root, 'wait-result.json');

  await assert.rejects(
    execFileAsync(
      process.execPath,
      [
        cliPath,
        'launch-wait',
        '--workspace',
        root,
        '--exe',
        path.join(root, 'app.exe'),
        '--instance',
        'callback-disabled',
        '--app-data-dir',
        path.join(root, 'app-data'),
        '--callback-spec',
        path.join(root, 'callback.json'),
        '--watcher-log',
        watcherLog,
        '--launch-out',
        launchOut,
        '--out',
        waitResult,
      ],
      { windowsHide: true },
    ),
    /callback-spec is disabled.*hidden CLI turn/u,
  );

  assert.equal(await exists(watcherLog), false);
  assert.equal(await exists(launchOut), false);
  assert.equal(await exists(waitResult), false);
});

test('foreground wait also rejects the unsupported callback route', async () => {
  await assert.rejects(
    execFileAsync(
      process.execPath,
      [cliPath, 'wait', '--callback-spec', 'C:\\evidence\\callback.json'],
      { windowsHide: true },
    ),
    /callback-spec is disabled.*not a desktop wake transport/u,
  );
});

async function exists(filePath) {
  try {
    await access(filePath);
    return true;
  } catch {
    return false;
  }
}
