import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const stateFile =
  process.env.RUNTIME_STATUS_FILE ?? path.join(process.cwd(), '.dev', 'runtime-status.json');
const statusUrl = process.env.RUNTIME_STATUS_URL ?? 'http://127.0.0.1:41415/mark-stale';
const input = parseArgs(process.argv.slice(2));
const staleTargets = normalizeTargets(input.target);
const nextState = {
  statusVersion: 1,
  stale: true,
  staleTargets,
  ...(input.reason ? { reason: input.reason } : {}),
  generation: new Date().toISOString(),
  markedAt: new Date().toISOString(),
};

if (!(await postStatus(nextState))) {
  await writeFallbackState(nextState);
}

console.log(`Marked ${staleTargets.join('/')} stale${input.reason ? `: ${input.reason}` : ''}`);

function parseArgs(args) {
  const parsed = {};

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];

    if (arg === '--target' || arg === '--targets') {
      parsed.target = args[index + 1];
      index += 1;
    } else if (arg.startsWith('--target=')) {
      parsed.target = arg.slice('--target='.length);
    } else if (arg === '--reason') {
      parsed.reason = args[index + 1];
      index += 1;
    } else if (arg.startsWith('--reason=')) {
      parsed.reason = arg.slice('--reason='.length);
    }
  }

  return parsed;
}

function normalizeTargets(value) {
  if (!value) {
    return ['app'];
  }

  const targets = String(value)
    .split(',')
    .map((entry) => entry.trim())
    .filter((entry) => entry === 'app' || entry === 'frontend' || entry === 'backend');

  return targets.length > 0 ? [...new Set(targets)] : ['app'];
}

async function postStatus(state) {
  try {
    const response = await fetch(statusUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        target: state.staleTargets.join(','),
        reason: state.reason,
      }),
    });

    return response.ok;
  } catch {
    return false;
  }
}

async function writeFallbackState(state) {
  const previous = await readPreviousState();
  await mkdir(path.dirname(stateFile), { recursive: true });
  await writeFile(
    stateFile,
    `${JSON.stringify(
      {
        ...previous,
        ...state,
      },
      null,
      2,
    )}\n`,
    'utf8',
  );
}

async function readPreviousState() {
  try {
    return JSON.parse(await readFile(stateFile, 'utf8'));
  } catch {
    return {};
  }
}
