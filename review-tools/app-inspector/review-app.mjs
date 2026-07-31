#!/usr/bin/env node

import { access, mkdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

import { inspectDatabaseState } from './src/database-adapter.mjs';
import { compareSnapshots } from './src/snapshot-compare.mjs';
import { inspectSourceAndExecutable } from './src/source-adapter.mjs';
import { inspectStatusEndpoint } from './src/status-adapter.mjs';
import { inspectWindowsApplication } from './src/windows-adapter.mjs';
import { observationSignals, waitForChange } from './src/wait-for-change.mjs';
import { launchDetachedWait } from './src/background-launcher.mjs';
import { writeDurableJson, writeDurableText } from './src/durable-file.mjs';

const toolRoot = path.dirname(fileURLToPath(import.meta.url));

async function main() {
  const rawArguments = process.argv.slice(2);
  const { command, options } = parseArguments(rawArguments);

  if (command === 'help') {
    process.stdout.write(helpText());
    return;
  }

  if (command === 'compare') {
    const beforePath = requiredPath(options.before, '--before');
    const afterPath = requiredPath(options.after, '--after');
    const before = JSON.parse(await readFile(beforePath, 'utf8'));
    const after = JSON.parse(await readFile(afterPath, 'utf8'));
    const result = compareSnapshots(before, after, { beforePath, afterPath });
    await emitResult(result, options);
    return;
  }

  if (command === 'launch-wait') {
    rejectUnsupportedCallback(options);
    const result = await runDetachedLaunch(options, rawArguments.slice(1));
    process.stdout.write(
      options.format === 'human'
        ? `${humanSummary(result)}\n`
        : `${JSON.stringify(result, null, 2)}\n`,
    );
    return;
  }

  if (command === 'wait') {
    rejectUnsupportedCallback(options);
    const { result, exitCode } = await runWait(options);
    await emitResult(result, options);
    process.exitCode = exitCode;
    return;
  }

  if (command !== 'inspect') {
    throw new Error(`Unknown command: ${command}. Run with --help for usage.`);
  }

  const context = await resolveContext(options);
  const observedAt = new Date().toISOString();
  const screenshotPath =
    options.screenshot === false
      ? undefined
      : path.join(context.evidenceRoot, `window-${safeTimestamp(observedAt)}.png`);

  const result = await captureSnapshot(context, { observedAt, screenshotPath });
  await emitResult(result, options);
}

async function captureSnapshot(context, supplied = {}) {
  if (supplied.screenshotPath) {
    await mkdir(path.dirname(supplied.screenshotPath), { recursive: true });
  }

  const [windows, identity, status] = await Promise.all([
    completeWindowsObservation(context, supplied.windows, supplied.screenshotPath),
    inspectSourceAndExecutable({
      workspaceRoot: context.workspaceRoot,
      executablePath: context.executablePath,
    }),
    inspectStatusEndpoint(context.statusUrl),
  ]);

  const appDataDir = context.appDataDir ?? defaultAppDataDir(identity);
  const databasePath =
    context.databasePath ?? path.join(appDataDir, 'codex-orchestrator-active-v3.sqlite');
  const state = supplied.state ?? (await inspectDatabaseState(databasePath));

  return {
    schemaVersion: 'review-app-observation/v1',
    observedAt: supplied.observedAt ?? new Date().toISOString(),
    instance: context.instance,
    request: {
      workspaceRoot: context.workspaceRoot,
      executablePath: context.executablePath,
      appDataDir,
      databasePath,
      statusUrl: context.statusUrl,
    },
    application: {
      process: windows.process,
      window: windows.window,
      screenshot: windows.screenshot,
      accessibility: windows.accessibility,
      nativeAdapterDiagnostics: windows.diagnostics,
      visibleRoute: unavailable(
        'The running production WebView2 host has no review attachment endpoint. Window capture is observed, but a route or semantic DOM is not exposed.',
      ),
      executable: identity.executable,
      product: identity.product,
      source: identity.source,
      sourceToExecutable: inferred(
        'The executable is located under the requested workspace. No embedded build provenance proves which Git commit produced it.',
        {
          workspaceContainsExecutable: isWithin(context.workspaceRoot, context.executablePath),
        },
      ),
      developmentStatusEndpoint: status,
      durableState: state,
    },
    boundaries: {
      readOnly: true,
      productCompositionChanged: false,
      interactionPerformed: false,
      credentialAccessed: false,
      semanticScreenInspection:
        'unavailable for this already-running release process; use the observed screenshot for visual review',
    },
  };
}

async function completeWindowsObservation(context, polledWindows, screenshotPath) {
  if (!polledWindows || polledWindows.screenshot?.disposition !== 'observed') {
    return inspectWindowsApplication({
      toolRoot,
      executablePath: context.executablePath,
      pid: context.pid,
      screenshotPath,
    });
  }

  const complete = await inspectWindowsApplication({
    toolRoot,
    executablePath: context.executablePath,
    pid: context.pid,
  });
  return {
    ...complete,
    screenshot: polledWindows.screenshot,
  };
}

async function runWait(options) {
  const context = await resolveContext(options);
  const condition = options.condition ?? 'either';
  const pollMs = positiveInteger(options['poll-ms'] ?? '500', '--poll-ms');
  const stableObservations = positiveInteger(
    options['stable-observations'] ?? '3',
    '--stable-observations',
  );
  if (stableObservations < 2) {
    throw new Error('--stable-observations must be at least 2.');
  }
  const timeoutMs = positiveInteger(options['timeout-ms'] ?? '300000', '--timeout-ms');
  const cancelFilePath = options['cancel-file'] ? path.resolve(options['cancel-file']) : null;
  const stamp = safeTimestamp(new Date().toISOString());
  const beforePath = path.resolve(
    options['before-out'] ??
      options.before ??
      path.join(context.evidenceRoot, `wait-before-${stamp}.json`),
  );
  const afterPath = path.resolve(
    options['after-out'] ?? path.join(context.evidenceRoot, `wait-after-${stamp}.json`),
  );
  const comparisonPath = path.resolve(
    options['comparison-out'] ?? path.join(context.evidenceRoot, `wait-comparison-${stamp}.json`),
  );
  const humanPath = path.resolve(
    options['human-out'] ?? path.join(context.evidenceRoot, `wait-summary-${stamp}.txt`),
  );
  const beforeScreenshotPath =
    options.screenshot === false
      ? undefined
      : path.resolve(
          options['before-screenshot-out'] ??
            path.join(context.evidenceRoot, `wait-before-${stamp}.png`),
        );
  const afterScreenshotPath =
    options.screenshot === false
      ? undefined
      : path.resolve(
          options['after-screenshot-out'] ??
            path.join(context.evidenceRoot, `wait-after-${stamp}.png`),
        );
  assertDistinctPaths({
    suppliedBefore: options.before ? path.resolve(options.before) : null,
    retainedBefore: options['before-out'] || !options.before ? beforePath : null,
    afterSnapshot: afterPath,
    comparison: comparisonPath,
    humanSummary: humanPath,
    beforeScreenshot: beforeScreenshotPath,
    afterScreenshot: afterScreenshotPath,
    waitResult: options.out ? path.resolve(options.out) : null,
    cancelFile: cancelFilePath,
  });
  let before;
  let baselineSource;
  if (options.before) {
    const suppliedPath = path.resolve(options.before);
    before = JSON.parse(await readFile(suppliedPath, 'utf8'));
    if (before?.schemaVersion !== 'review-app-observation/v1' || !before.application) {
      throw new Error('--before must contain a review-app-observation/v1 snapshot.');
    }
    baselineSource = 'supplied';
    if (options['before-out']) await writeJson(beforePath, before);
  } else {
    before = await captureSnapshot(context, { screenshotPath: beforeScreenshotPath });
    baselineSource = 'captured';
    await writeJson(beforePath, before);
  }

  const baseline = observationSignals(before);
  const controller = new globalThis.AbortController();
  const cancel = () => controller.abort();
  process.once('SIGINT', cancel);
  process.once('SIGTERM', cancel);

  let waited;
  try {
    waited = await waitForChange({
      baseline,
      condition,
      pollMs,
      stableObservations,
      timeoutMs,
      signal: controller.signal,
      isCancelled: () => fileExists(cancelFilePath),
      observe: (observationOptions) =>
        observeWaitSignals(context, condition, afterScreenshotPath, observationOptions),
    });
  } finally {
    process.removeListener('SIGINT', cancel);
    process.removeListener('SIGTERM', cancel);
  }

  const last = waited.lastObservation;
  const after = await captureSnapshot(context, {
    observedAt: last?.observedAt,
    screenshotPath: afterScreenshotPath,
    windows: last?.windows,
    state: last?.state,
  });
  const comparison = compareSnapshots(before, after, {
    beforePath:
      baselineSource === 'supplied' && !options['before-out']
        ? path.resolve(options.before)
        : beforePath,
    afterPath,
  });
  await writeJson(afterPath, after);
  await writeJson(comparisonPath, comparison);

  const result = {
    schemaVersion: 'review-app-wait/v1',
    status: waited.status,
    condition,
    trigger: waited.trigger ?? null,
    startedAt: waited.startedAt,
    finishedAt: waited.finishedAt,
    elapsedMs: waited.elapsedMs,
    observations: waited.observationCount,
    stability: {
      requiredObservations: stableObservations,
      matchingObservations: waited.stableCount,
      pollMs,
    },
    baseline: { source: baselineSource, signals: baseline },
    finalSignals: observationSignals(after),
    comparison: {
      changed: comparison.changed,
      ...comparison.summary,
      changes: comparison.changes,
    },
    artifacts: {
      beforeSnapshot:
        baselineSource === 'supplied' && !options['before-out']
          ? path.resolve(options.before)
          : beforePath,
      afterSnapshot: afterPath,
      comparison: comparisonPath,
      humanSummary: humanPath,
      waitResult: options.out ? path.resolve(options.out) : null,
    },
    callback: {
      configured: false,
      state: 'disabled_unsupported_desktop_wake',
      reason:
        'No supported deterministic transport can queue a turn in an existing Codex desktop task from this process.',
    },
    cancellation: {
      signalHandlers: ['SIGINT', 'SIGTERM'],
      cancelFile: cancelFilePath,
    },
    boundaries: {
      readOnly: true,
      interactionPerformed: false,
      productCompositionChanged: false,
    },
  };
  result.humanOutput = [
    humanSummary(result),
    '',
    'Final snapshot:',
    humanSummary(after, afterPath),
  ].join('\n');
  await writeText(humanPath, `${result.humanOutput}\n`);

  return {
    result,
    exitCode: waited.status === 'completed' ? 0 : waited.status === 'timed_out' ? 2 : 130,
  };
}

async function runDetachedLaunch(options, rawWaitArguments) {
  const context = await resolveContext(options);
  if (!options.out) throw new Error('--out is required for launch-wait.');
  const timeoutMs = positiveInteger(options['timeout-ms'] ?? '300000', '--timeout-ms');
  const cancelFilePath = path.resolve(
    options['cancel-file'] ?? path.join(context.evidenceRoot, 'watcher.cancel'),
  );
  if (await fileExists(cancelFilePath)) {
    throw new Error(`Cancel file already exists; remove it before launch: ${cancelFilePath}`);
  }
  const watcherLogPath = path.resolve(
    options['watcher-log'] ?? path.join(context.evidenceRoot, 'watcher.log'),
  );
  const launchResultPath = path.resolve(
    options['launch-out'] ?? path.join(context.evidenceRoot, 'watcher-launch.json'),
  );
  const stamp = safeTimestamp(new Date().toISOString());
  const detachedEvidence = {
    suppliedBefore: options.before ? path.resolve(options.before) : null,
    retainedBefore: options['before-out']
      ? path.resolve(options['before-out'])
      : options.before
        ? null
        : path.join(context.evidenceRoot, `wait-before-${stamp}.json`),
    afterSnapshot: path.resolve(
      options['after-out'] ?? path.join(context.evidenceRoot, `wait-after-${stamp}.json`),
    ),
    comparison: path.resolve(
      options['comparison-out'] ?? path.join(context.evidenceRoot, `wait-comparison-${stamp}.json`),
    ),
    humanSummary: path.resolve(
      options['human-out'] ?? path.join(context.evidenceRoot, `wait-summary-${stamp}.txt`),
    ),
    beforeScreenshot:
      options.screenshot === false
        ? null
        : path.resolve(
            options['before-screenshot-out'] ??
              path.join(context.evidenceRoot, `wait-before-${stamp}.png`),
          ),
    afterScreenshot:
      options.screenshot === false
        ? null
        : path.resolve(
            options['after-screenshot-out'] ??
              path.join(context.evidenceRoot, `wait-after-${stamp}.png`),
          ),
  };
  assertDistinctPaths({
    watcherLog: watcherLogPath,
    launchResult: launchResultPath,
    ...detachedEvidence,
    waitResult: path.resolve(options.out),
    cancelFile: cancelFilePath,
  });
  const waitArguments = removeLauncherArguments(rawWaitArguments);
  if (!options['cancel-file']) waitArguments.push('--cancel-file', cancelFilePath);
  if (!options.before && !options['before-out']) {
    waitArguments.push('--before-out', detachedEvidence.retainedBefore);
  }
  appendMissingPath(waitArguments, options, 'after-out', detachedEvidence.afterSnapshot);
  appendMissingPath(waitArguments, options, 'comparison-out', detachedEvidence.comparison);
  appendMissingPath(waitArguments, options, 'human-out', detachedEvidence.humanSummary);
  if (options.screenshot !== false) {
    appendMissingPath(
      waitArguments,
      options,
      'before-screenshot-out',
      detachedEvidence.beforeScreenshot,
    );
    appendMissingPath(
      waitArguments,
      options,
      'after-screenshot-out',
      detachedEvidence.afterScreenshot,
    );
  }
  return launchDetachedWait({
    scriptPath: fileURLToPath(import.meta.url),
    waitArguments,
    workspaceRoot: context.workspaceRoot,
    watcherLogPath,
    launchResultPath,
    waitResultPath: options.out,
    evidenceRoot: context.evidenceRoot,
    timeoutMs,
    cancelFilePath,
  });
}

function rejectUnsupportedCallback(options) {
  if (!options['callback-spec']) return;
  throw new Error(
    '--callback-spec is disabled: no supported deterministic transport can queue a turn in an existing Codex desktop task. `codex exec resume` starts a separate hidden CLI turn and is not a desktop wake transport.',
  );
}

function removeLauncherArguments(arguments_) {
  const result = [];
  for (let index = 0; index < arguments_.length; index += 1) {
    const token = arguments_[index];
    if (token === '--watcher-log' || token === '--launch-out') {
      index += 1;
      continue;
    }
    result.push(token);
  }
  return result;
}

function appendMissingPath(arguments_, options, name, value) {
  if (!options[name]) arguments_.push(`--${name}`, value);
}

async function fileExists(filePath) {
  if (!filePath) return false;
  try {
    await access(filePath);
    return true;
  } catch {
    return false;
  }
}

function assertDistinctPaths(entries) {
  const seen = new Map();
  for (const [name, filePath] of Object.entries(entries)) {
    if (!filePath) continue;
    const normalized = path.resolve(filePath).toLowerCase();
    const existing = seen.get(normalized);
    if (existing) throw new Error(`${name} must not reuse the ${existing} path: ${filePath}`);
    seen.set(normalized, name);
  }
}

async function observeWaitSignals(context, condition, screenshotPath, observationOptions) {
  const observeVisual = condition === 'visual' || condition === 'either';
  const observeDurable = condition === 'durable' || condition === 'either';
  const observedAt = new Date().toISOString();
  const [windows, state] = await Promise.all([
    observeVisual
      ? inspectWindowsApplication({
          toolRoot,
          executablePath: context.executablePath,
          pid: context.pid,
          screenshotPath,
          skipAccessibility: true,
          signal: observationOptions.signal,
          timeoutMs: observationOptions.timeoutMs,
        })
      : null,
    observeDurable ? inspectDatabaseState(context.databasePath) : null,
  ]);
  return {
    observedAt,
    visual: windows?.screenshot?.value?.sha256 ?? null,
    durable: state?.value?.fingerprint ?? null,
    windows,
    state,
  };
}

async function resolveContext(options) {
  const workspaceRoot = path.resolve(requiredPath(options.workspace, '--workspace'));
  const executablePath = path.resolve(
    options.exe ??
      path.join(workspaceRoot, 'src-tauri', 'target', 'release', 'codex-orchestrator.exe'),
  );
  const instance = options.instance ?? 'explicit-release';
  const evidenceRoot = path.resolve(
    options['evidence-root'] ?? path.join(workspaceRoot, '.dev', 'review-app-inspector', instance),
  );
  let appDataDir = options['app-data-dir'] ? path.resolve(options['app-data-dir']) : undefined;
  if (!appDataDir) {
    const identity = await inspectSourceAndExecutable({ workspaceRoot, executablePath });
    appDataDir = defaultAppDataDir(identity);
  }
  return {
    workspaceRoot,
    executablePath,
    instance,
    evidenceRoot,
    appDataDir,
    databasePath: options.database
      ? path.resolve(options.database)
      : path.join(appDataDir, 'codex-orchestrator-active-v3.sqlite'),
    statusUrl: options['status-url'] ?? 'http://127.0.0.1:41415',
    pid: optionalInteger(options.pid, '--pid'),
  };
}

function defaultAppDataDir(identity) {
  return path.resolve(
    path.join(
      process.env.APPDATA ?? '',
      identity.product.value?.identifier ?? 'dev.codex-orchestrator.app',
    ),
  );
}

function parseArguments(arguments_) {
  const options = {};
  let command = 'inspect';
  let index = 0;

  if (arguments_[0] && !arguments_[0].startsWith('-')) {
    command = arguments_[0];
    index = 1;
  }

  for (; index < arguments_.length; index += 1) {
    const token = arguments_[index];
    if (token === '--help' || token === '-h') {
      return { command: 'help', options };
    }
    if (token === '--no-screenshot') {
      options.screenshot = false;
      continue;
    }
    if (!token.startsWith('--')) {
      throw new Error(`Unexpected argument: ${token}`);
    }
    const key = token.slice(2);
    const value = arguments_[index + 1];
    if (!value || value.startsWith('--')) {
      throw new Error(`Missing value for ${token}`);
    }
    options[key] = value;
    index += 1;
  }

  return { command, options };
}

async function emitResult(result, options) {
  if (options.out) {
    const outputPath = path.resolve(options.out);
    await writeDurableJson(outputPath, result);
  }

  if (options.format === 'human') {
    process.stdout.write(`${humanSummary(result, options.out)}\n`);
    return;
  }

  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
}

async function writeJson(outputPath, value) {
  await writeDurableJson(outputPath, value);
}

async function writeText(outputPath, value) {
  await writeDurableText(outputPath, value);
}

function humanSummary(result, outputPath) {
  if (result.schemaVersion === 'review-app-wait-launch/v1') {
    return [
      `Watcher PID: ${result.watcherPid}`,
      `Timeout: ${result.watcher.timeoutMs} ms`,
      `Watcher log: ${result.artifacts.watcherLog}`,
      `Wait result: ${result.artifacts.waitResult}`,
      'Desktop callback: disabled (unsupported host transport)',
      `Launch record: ${result.artifacts.launchResult}`,
      `Cancel: ${result.cancellation.powershell}`,
      `Cleanup: ${result.cleanup.instruction}`,
      `Cleanup target: ${result.cleanup.target}`,
    ].join('\n');
  }

  if (result.schemaVersion === 'review-app-wait/v1') {
    if (result.humanOutput) return result.humanOutput;
    const status =
      result.status === 'completed'
        ? `completed (${result.trigger.kind} changed)`
        : result.status.replace('_', ' ');
    const lines = [
      `Wait: ${status}`,
      `Condition: ${result.condition}`,
      `Elapsed: ${result.elapsedMs} ms across ${result.observations} observations`,
      `Stability: ${result.stability.matchingObservations}/${result.stability.requiredObservations} matching observations`,
      `Durable state: ${result.comparison.durableStateChanged ? 'changed' : 'unchanged'}`,
      `Screenshot: ${result.comparison.screenshotChanged ? 'changed' : 'unchanged or unavailable'}`,
      `Before snapshot: ${result.artifacts.beforeSnapshot}`,
      `After snapshot: ${result.artifacts.afterSnapshot}`,
      `Comparison JSON: ${result.artifacts.comparison}`,
      `Human summary: ${result.artifacts.humanSummary}`,
      `Callback: ${result.callback.state}`,
    ];
    for (const change of result.comparison.changes.slice(0, 20)) {
      lines.push(`- ${change.path}: ${compact(change.before)} -> ${compact(change.after)}`);
    }
    if (result.comparison.changes.length > 20) {
      lines.push(`- ... ${result.comparison.changes.length - 20} more changes in JSON output`);
    }
    return lines.join('\n');
  }

  if (result.schemaVersion === 'review-app-comparison/v1') {
    const lines = [
      `Changed: ${result.changed ? 'yes' : 'no'}`,
      `Durable state: ${result.summary.durableStateChanged ? 'changed' : 'unchanged'}`,
      `Screenshot: ${result.summary.screenshotChanged ? 'changed' : 'unchanged or unavailable'}`,
    ];
    for (const change of result.changes.slice(0, 20)) {
      lines.push(`- ${change.path}: ${compact(change.before)} -> ${compact(change.after)}`);
    }
    if (result.changes.length > 20) {
      lines.push(`- ... ${result.changes.length - 20} more changes in JSON output`);
    }
    return lines.join('\n');
  }

  const app = result.application;
  const state = app.durableState.value;
  const lines = [
    `Instance: ${result.instance}`,
    `Process: ${app.process.disposition === 'observed' ? `running (PID ${app.process.value.pid})` : app.process.reason}`,
    `Executable: ${app.executable.disposition === 'observed' ? app.executable.value.path : app.executable.reason}`,
    `Product version: ${app.product.value?.version ?? 'unavailable'}`,
    `Source HEAD: ${app.source.value?.head ?? 'unavailable'} (${app.source.value?.dirty ? 'dirty' : 'clean'})`,
    `Window: ${app.window.value?.title ?? app.window.reason ?? 'unavailable'}`,
    `Visible route: unavailable (the production host has no attachment seam)`,
    `Screenshot: ${app.screenshot.value?.path ?? app.screenshot.reason ?? 'unavailable'}`,
    `Status endpoint: ${app.developmentStatusEndpoint.disposition}`,
    `Database: ${app.durableState.disposition === 'observed' ? app.durableState.source : app.durableState.reason}`,
  ];
  if (state) {
    lines.push(
      `Durable overview: ${state.planningDrafts.length} drafts, ${state.initiatedEpics.length} initiated epics, ${state.initiatedSprints.length} planned sprints`,
      `Recent invocations: ${state.recentInvocations.map((item) => `${item.title}=${item.status}`).join(', ') || 'none'}`,
      `State fingerprint: ${state.fingerprint}`,
    );
  }
  if (outputPath) {
    lines.push(`Snapshot JSON: ${path.resolve(outputPath)}`);
  }
  return lines.join('\n');
}

function requiredPath(value, name) {
  if (!value || typeof value !== 'string') {
    throw new Error(`${name} is required.`);
  }
  return value;
}

function optionalInteger(value, name) {
  if (value === undefined) return undefined;
  const parsed = Number.parseInt(value, 10);
  if (!Number.isSafeInteger(parsed) || parsed <= 0) {
    throw new Error(`${name} must be a positive process ID.`);
  }
  return parsed;
}

function positiveInteger(value, name) {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isSafeInteger(parsed) || parsed <= 0 || String(parsed) !== String(value)) {
    throw new Error(`${name} must be a positive integer.`);
  }
  return parsed;
}

function safeTimestamp(value) {
  return value.replaceAll(':', '-').replaceAll('.', '-');
}

function isWithin(root, candidate) {
  const relative = path.relative(path.resolve(root), path.resolve(candidate));
  return relative === '' || (!relative.startsWith('..') && !path.isAbsolute(relative));
}

function compact(value) {
  const serialized = JSON.stringify(value);
  return serialized && serialized.length > 100 ? `${serialized.slice(0, 97)}...` : serialized;
}

function inferred(reason, value) {
  return { disposition: 'inferred', reason, value };
}

function unavailable(reason) {
  return { disposition: 'unavailable', reason };
}

function helpText() {
  return `Codex Orchestrator provisional review companion

Inspect an explicitly identified running application:
  node review-tools/app-inspector/review-app.mjs inspect --workspace <absolute-path> --exe <absolute-path> --instance <name> --out <snapshot.json>

Compare two retained observations:
  node review-tools/app-inspector/review-app.mjs compare --before <snapshot.json> --after <snapshot.json>

Wait for a stable observed change without interacting with the application:
  node review-tools/app-inspector/review-app.mjs wait --workspace <absolute-path> --exe <absolute-path> --instance <name> --condition <visual|durable|either> --timeout-ms <milliseconds>

Launch the same wait as a detached watcher that finalizes evidence only:
  node review-tools/app-inspector/review-app.mjs launch-wait --workspace <absolute-path> --exe <absolute-path> --instance <name> --watcher-log <absolute-path> --launch-out <absolute-path> --out <wait-result.json>

Options:
  --pid <id>                 Require this exact process ID.
  --app-data-dir <path>      Explicit Tauri application-data directory.
  --database <path>          Explicit active-v3 SQLite database.
  --status-url <url>         Development status-server base URL.
  --evidence-root <path>     Screenshot output directory.
  --no-screenshot            Skip native window capture.
  --condition <kind>         Wait for visual, durable, or either change (default: either).
  --before <path>            Use a supplied observation snapshot as the wait baseline.
  --before-out <path>        Save a captured or copied baseline snapshot here.
  --before-screenshot-out <path> Save the baseline render here.
  --after-out <path>         Save the complete final observation snapshot here.
  --after-screenshot-out <path> Save the final render here.
  --comparison-out <path>    Save the before/after comparison JSON here.
  --human-out <path>         Save the wait result and artifact paths as readable text.
  --poll-ms <milliseconds>   Poll interval (default: 500).
  --stable-observations <n>  Identical changed observations required (default: 3; minimum: 2).
  --timeout-ms <milliseconds> Bound the wait (default: 300000; timeout exits 2).
  --cancel-file <path>       Graceful detached cancellation signal file.
  --callback-spec <path>     Disabled: no supported desktop-task wake transport is available.
  --watcher-log <path>       Detached watcher stdout/stderr log (launch-wait only).
  --launch-out <path>        Detached watcher PID and cancellation record (launch-wait only).
  --format human             Print a compact human summary; JSON is the default.
  --out <path>               Also retain the machine-readable result.
`;
}

main().catch((error) => {
  process.stderr.write(`review-app: ${message(error)}\n`);
  process.exitCode = 1;
});

function message(error) {
  return error instanceof Error ? error.message : String(error);
}
