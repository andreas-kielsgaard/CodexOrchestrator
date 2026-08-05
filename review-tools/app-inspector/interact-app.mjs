#!/usr/bin/env node

import { mkdir } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import { execFile } from 'node:child_process';

const execFileAsync = promisify(execFile);
const toolRoot = path.dirname(fileURLToPath(import.meta.url));
const prefix = 'REVIEW_APP_INTERACTION_V1:';

async function main() {
  const { action, options } = parseArguments(process.argv.slice(2));
  if (action === 'help') return process.stdout.write(helpText());
  const executablePath = requiredPath(options.exe, '--exe');
  const pid = positiveInteger(options.pid, '--pid');
  const x = nonNegativeInteger(options.x, '--x');
  const y = nonNegativeInteger(options.y, '--y');
  if (options.text || options['text-file']) {
    throw new Error(
      'This adapter has click-only authority; use the owned WebView control adapter for typing.',
    );
  }
  const result = await interact({ executablePath, pid, action, x, y });
  const receipt = {
    schemaVersion: 'review-app-interaction/v1',
    observedAt: new Date().toISOString(),
    request: {
      executablePath: path.resolve(executablePath),
      pid,
      action,
      clientPoint: { x, y },
    },
    result,
    boundaries: {
      foregrounded: false,
      semanticOutcome:
        'not_observed; capture a separate durable or visual observation after this transport receipt',
      productCompositionChanged: false,
    },
  };
  if (options.out) await writeReceipt(path.resolve(options.out), receipt);
  process.stdout.write(`${JSON.stringify(receipt, null, 2)}\n`);
}

async function interact({ executablePath, pid, action, x, y }) {
  if (process.platform !== 'win32')
    throw new Error('The desktop interaction adapter currently supports Windows only.');
  const script = path.join(toolRoot, 'adapters', 'windows-interaction.ps1');
  const { stdout } = await execFileAsync(
    'powershell.exe',
    [
      '-NoProfile',
      '-ExecutionPolicy',
      'Bypass',
      '-File',
      script,
      '-ExecutablePath',
      executablePath,
      '-ProcessId',
      String(pid),
      '-Action',
      action,
      '-X',
      String(x),
      '-Y',
      String(y),
    ],
    {
      encoding: 'utf8',
      windowsHide: true,
      timeout: 15_000,
      env: process.env,
    },
  );
  return parseInteractionOutput(stdout);
}

export function parseInteractionOutput(stdout) {
  const frames = String(stdout ?? '')
    .split(/\r?\n/u)
    .filter((line) => line.startsWith(prefix));
  if (frames.length !== 1)
    throw new Error(
      `Interaction adapter expected exactly one ${prefix} frame; observed ${frames.length}.`,
    );
  const encoded = frames[0].slice(prefix.length);
  if (!/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/u.test(encoded))
    throw new Error('Interaction adapter frame is not canonical base64.');
  const bytes = Buffer.from(encoded, 'base64');
  if (bytes.toString('base64') !== encoded)
    throw new Error('Interaction adapter frame is not canonical base64.');
  const value = JSON.parse(bytes.toString('utf8'));
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('Interaction adapter frame must contain a JSON object.');
  return value;
}

function parseArguments(args) {
  const [action = 'help', ...rest] = args;
  if (action === '--help' || action === '-h') return { action: 'help', options: {} };
  if (action !== 'click') throw new Error(`Unknown action: ${action}. Run with --help for usage.`);
  const options = {};
  for (let index = 0; index < rest.length; index += 1) {
    const token = rest[index];
    if (!token.startsWith('--')) throw new Error(`Unexpected argument: ${token}`);
    const key = token.slice(2);
    const value = rest[index + 1];
    if (!value || value.startsWith('--')) throw new Error(`Missing value for ${token}`);
    options[key] = value;
    index += 1;
  }
  return { action, options };
}

function requiredPath(value, name) {
  if (!value) throw new Error(`${name} is required.`);
  return value;
}
function positiveInteger(value, name) {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isSafeInteger(parsed) || parsed <= 0 || String(parsed) !== String(value))
    throw new Error(`${name} must be a positive integer.`);
  return parsed;
}
function nonNegativeInteger(value, name) {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isSafeInteger(parsed) || parsed < 0 || String(parsed) !== String(value))
    throw new Error(`${name} must be a non-negative integer.`);
  return parsed;
}
async function writeReceipt(filePath, receipt) {
  const { writeFile, rename } = await import('node:fs/promises');
  await mkdir(path.dirname(filePath), { recursive: true });
  const temporary = `${filePath}.tmp`;
  await writeFile(temporary, `${JSON.stringify(receipt, null, 2)}\n`, 'utf8');
  await rename(temporary, filePath);
}
function helpText() {
  return `Codex Orchestrator development desktop interaction companion\n\nTarget one exact running Windows application without foregrounding it:\n  node review-tools/app-inspector/interact-app.mjs click --exe <absolute-path> --pid <pid> --x <client-x> --y <client-y>\n\nCoordinates are relative to the selected application's main-window client area and must be from 0 through 32767. The target HWND must belong to the selected process or a live descendant. This adapter has click-only authority and does not read or write the clipboard. The receipt proves only that the target child window acknowledged the explicit mouse-down and mouse-up messages. It does not prove UI, provider, or application semantics; retain a separate visual or durable observation.\n`;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    process.stderr.write(
      `interact-app: ${error instanceof Error ? error.message : String(error)}\n`,
    );
    process.exitCode = 1;
  });
}
