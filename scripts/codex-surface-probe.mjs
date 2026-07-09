#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import readline from 'node:readline';
import process from 'node:process';

const options = parseArgs(process.argv.slice(2));
const cwd = path.resolve(options.cwd ?? process.cwd());
const outputDir = path.resolve(
  options.out ?? path.join('.dev', 'codex-surface-probes', timestampForPath(new Date())),
);
const codexCommand = options.codexCommand ?? 'codex';
const prompt = options.prompt ?? 'Reply with exactly: codex-surface-probe-ok';
const timeoutMs = Number.parseInt(options.timeoutMs ?? '120000', 10);

await mkdir(outputDir, { recursive: true });

const summary = {
  startedAt: new Date().toISOString(),
  cwd,
  outputDir,
  codexCommand,
  prompt,
  timeoutMs,
  appServer: options.skipAppServer
    ? { status: 'skipped', reason: '--skip-app-server was provided' }
    : await captureFailure(() =>
        runAppServerProbe({ codexCommand, cwd, prompt, timeoutMs, model: options.model }),
      ),
  exec: options.skipExec
    ? { status: 'skipped', reason: '--skip-exec was provided' }
    : await captureFailure(() =>
        runExecProbe({ codexCommand, cwd, prompt, timeoutMs, model: options.model }),
      ),
  completedAt: new Date().toISOString(),
};

await writeJson(path.join(outputDir, 'summary.json'), summary);
console.log(JSON.stringify(summary, null, 2));

async function runAppServerProbe(input) {
  const clientMessages = [];
  const serverMessages = [];
  const stderrChunks = [];
  const args = ['app-server', '--listen', 'stdio://'];
  const child = spawn(input.codexCommand, args, {
    cwd: input.cwd,
    stdio: ['pipe', 'pipe', 'pipe'],
    windowsHide: true,
  });
  const startedAt = new Date().toISOString();

  let threadId;
  let completed = false;
  let settle;
  const done = new Promise((resolve) => {
    settle = resolve;
  });
  const timeout = setTimeout(() => {
    settle({
      status: 'failed',
      statusReason: `Timed out after ${input.timeoutMs}ms`,
    });
  }, input.timeoutMs);

  child.once('error', (error) => {
    settle({ status: 'error', statusReason: error.message });
  });

  child.stderr.setEncoding('utf8');
  child.stderr.on('data', (chunk) => {
    stderrChunks.push(chunk);
  });

  const lines = readline.createInterface({ input: child.stdout });
  lines.on('line', (line) => {
    const receivedAt = new Date().toISOString();
    let message;

    try {
      message = JSON.parse(line);
    } catch (error) {
      serverMessages.push({ receivedAt, parseError: error.message, line });
      settle({ status: 'error', statusReason: `Invalid server JSON: ${error.message}` });
      return;
    }

    serverMessages.push({ receivedAt, message });

    if (message.id !== undefined && message.error !== undefined) {
      settle({
        status: 'failed',
        statusReason: `App-server response ${message.id} returned an error`,
      });
      return;
    }

    if (message.id === 1 && typeof message.result?.thread?.id === 'string') {
      threadId = message.result.thread.id;
      send({
        method: 'turn/start',
        id: 2,
        params: {
          threadId,
          input: [{ type: 'text', text: input.prompt }],
          cwd: input.cwd,
        },
      });
      return;
    }

    if (message.method === 'turn/completed') {
      completed = true;
      settle({
        status: 'completed',
        statusReason: 'App-server emitted turn/completed',
      });
    }
  });

  child.once('close', (exitCode, signal) => {
    if (!completed) {
      settle({
        status: 'failed',
        statusReason:
          signal === null
            ? `App-server process closed before turn completion with code ${exitCode}`
            : `App-server process closed before turn completion on signal ${signal}`,
      });
    }
  });

  send({
    method: 'initialize',
    id: 0,
    params: {
      clientInfo: {
        name: 'codex_orchestrator_probe',
        title: 'Codex Orchestrator Probe',
        version: '0.1.0',
      },
    },
  });
  send({ method: 'initialized', params: {} });
  send({
    method: 'thread/start',
    id: 1,
    params: input.model === undefined ? {} : { model: input.model },
  });

  const result = await done;
  clearTimeout(timeout);
  child.stdin.end();
  child.kill();

  const serverJsonl = serverMessages.map((entry) => JSON.stringify(entry)).join('\n');
  const clientJsonl = clientMessages.map((entry) => JSON.stringify(entry)).join('\n');
  await writeFile(path.join(outputDir, 'app-server-client.jsonl'), `${clientJsonl}\n`);
  await writeFile(path.join(outputDir, 'app-server-server.jsonl'), `${serverJsonl}\n`);
  await writeFile(path.join(outputDir, 'app-server-stderr.txt'), stderrChunks.join(''));

  return {
    ...result,
    startedAt,
    completedAt: new Date().toISOString(),
    command: input.codexCommand,
    args,
    threadId,
    clientMessageCount: clientMessages.length,
    serverMessageCount: serverMessages.length,
    stderrLength: stderrChunks.join('').length,
    serverSummary: summarizeAppServerMessages(serverMessages),
    artifacts: {
      clientJsonl: path.join(outputDir, 'app-server-client.jsonl'),
      serverJsonl: path.join(outputDir, 'app-server-server.jsonl'),
      stderr: path.join(outputDir, 'app-server-stderr.txt'),
    },
  };

  function send(message) {
    clientMessages.push({ sentAt: new Date().toISOString(), message });
    child.stdin.write(`${JSON.stringify(message)}\n`);
  }
}

async function runExecProbe(input) {
  const args = ['exec', '--json', '--ephemeral', '--sandbox', 'read-only'];

  if (input.model !== undefined) {
    args.push('--model', input.model);
  }

  args.push(input.prompt);

  const startedAt = new Date().toISOString();
  const result = await runProcess({
    command: input.codexCommand,
    args,
    cwd: input.cwd,
    timeoutMs: input.timeoutMs,
  });
  await writeFile(path.join(outputDir, 'exec-stdout.jsonl'), result.stdout);
  await writeFile(path.join(outputDir, 'exec-stderr.txt'), result.stderr);

  return {
    status: result.signal === null && result.exitCode === 0 ? 'completed' : 'failed',
    statusReason:
      result.signal === null
        ? `codex exec exited with code ${result.exitCode}`
        : `codex exec exited on signal ${result.signal}`,
    startedAt,
    completedAt: new Date().toISOString(),
    command: input.codexCommand,
    args,
    exitCode: result.exitCode,
    signal: result.signal,
    stdoutLength: result.stdout.length,
    stderrLength: result.stderr.length,
    jsonlSummary: summarizeJsonl(result.stdout),
    artifacts: {
      stdoutJsonl: path.join(outputDir, 'exec-stdout.jsonl'),
      stderr: path.join(outputDir, 'exec-stderr.txt'),
    },
  };
}

function runProcess(input) {
  return new Promise((resolve, reject) => {
    const child = spawn(input.command, input.args, {
      cwd: input.cwd,
      windowsHide: true,
      shell: false,
    });
    const timeout = setTimeout(() => {
      child.kill();
    }, input.timeoutMs);
    let stdout = '';
    let stderr = '';

    child.stdout.setEncoding('utf8');
    child.stdout.on('data', (chunk) => {
      stdout += chunk;
    });
    child.stderr.setEncoding('utf8');
    child.stderr.on('data', (chunk) => {
      stderr += chunk;
    });
    child.once('error', (error) => {
      clearTimeout(timeout);
      reject(error);
    });
    child.once('close', (exitCode, signal) => {
      clearTimeout(timeout);
      resolve({ stdout, stderr, exitCode, signal });
    });
  });
}

async function captureFailure(callback) {
  try {
    return await callback();
  } catch (error) {
    return {
      status: 'error',
      statusReason: error instanceof Error ? error.message : String(error),
    };
  }
}

function summarizeAppServerMessages(entries) {
  const notificationCountsByMethod = {};
  const tokenUsageUpdates = [];
  let threadId;
  let turnId;
  let terminalTurnStatus;
  let responseCount = 0;
  let errorResponseCount = 0;

  for (const entry of entries) {
    const message = entry.message;

    if (message === undefined) {
      continue;
    }

    if (message.id !== undefined) {
      responseCount += 1;
      if (message.error !== undefined) {
        errorResponseCount += 1;
      }
      threadId ??= readPath(message, ['result', 'thread', 'id']);
    }

    if (typeof message.method === 'string') {
      notificationCountsByMethod[message.method] =
        (notificationCountsByMethod[message.method] ?? 0) + 1;
      threadId ??=
        readPath(message, ['params', 'thread', 'id']) ?? readPath(message, ['params', 'threadId']);
      turnId ??=
        readPath(message, ['params', 'turn', 'id']) ?? readPath(message, ['params', 'turnId']);

      if (message.method === 'turn/completed') {
        terminalTurnStatus = message.params;
      }

      if (message.method === 'thread/tokenUsage/updated') {
        tokenUsageUpdates.push(message.params);
      }
    }
  }

  return {
    threadId,
    turnId,
    terminalTurnStatus,
    responseCount,
    errorResponseCount,
    notificationCountsByMethod,
    tokenUsageUpdates,
  };
}

function summarizeJsonl(jsonl) {
  const lines = jsonl.split(/\r\n|\n|\r/).filter((line) => line.trim() !== '');
  const typeCounts = {};
  let threadId;
  let terminalUsage;

  for (const line of lines) {
    let event;

    try {
      event = JSON.parse(line);
    } catch (error) {
      return { lineCount: lines.length, parseError: error.message };
    }

    if (typeof event.type === 'string') {
      typeCounts[event.type] = (typeCounts[event.type] ?? 0) + 1;
    }

    if (typeof event.thread_id === 'string') {
      threadId = event.thread_id;
    }

    if (event.type === 'turn.completed') {
      terminalUsage = event.usage;
    }
  }

  return { lineCount: lines.length, threadId, typeCounts, terminalUsage };
}

function readPath(value, pathSegments) {
  let current = value;

  for (const segment of pathSegments) {
    if (current === null || typeof current !== 'object' || Array.isArray(current)) {
      return undefined;
    }

    current = current[segment];
  }

  return typeof current === 'string' && current.length > 0 ? current : undefined;
}

async function writeJson(filePath, value) {
  await writeFile(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function parseArgs(args) {
  const parsed = {};

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];

    if (arg === '--skip-app-server') {
      parsed.skipAppServer = true;
      continue;
    }

    if (arg === '--skip-exec') {
      parsed.skipExec = true;
      continue;
    }

    if (!arg.startsWith('--')) {
      throw new Error(`Unexpected positional argument: ${arg}`);
    }

    const key = arg.slice(2).replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
    const value = args[index + 1];

    if (value === undefined || value.startsWith('--')) {
      throw new Error(`Missing value for ${arg}`);
    }

    parsed[key] = value;
    index += 1;
  }

  return parsed;
}

function timestampForPath(date) {
  return date.toISOString().replace(/[:.]/g, '-');
}
