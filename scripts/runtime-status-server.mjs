import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import path from 'node:path';
import { URL } from 'node:url';

const port = Number.parseInt(process.env.RUNTIME_STATUS_PORT ?? '41415', 10);
const host = process.env.RUNTIME_STATUS_HOST ?? '127.0.0.1';
const stateFile =
  process.env.RUNTIME_STATUS_FILE ?? path.join(process.cwd(), '.dev', 'runtime-status.json');
const serverStartedAt = new Date().toISOString();
const owner = runtimeOwner();

await ensureStateFile();

const server = createServer(async (request, response) => {
  response.setHeader('Access-Control-Allow-Origin', '*');
  response.setHeader('Access-Control-Allow-Methods', 'GET,POST,OPTIONS');
  response.setHeader('Access-Control-Allow-Headers', 'content-type');

  if (request.method === 'OPTIONS') {
    response.writeHead(204);
    response.end();
    return;
  }

  const requestUrl = new URL(request.url ?? '/', `http://${host}:${port}`);

  try {
    if (request.method === 'GET' && requestUrl.pathname === '/health') {
      writeJson(response, 200, { ok: true, serverStartedAt, owner });
      return;
    }

    if (request.method === 'GET' && requestUrl.pathname === '/status') {
      writeJson(response, 200, await readState());
      return;
    }

    if (request.method === 'POST' && requestUrl.pathname === '/mark-stale') {
      const input = await readJsonBody(request);
      const nextState = markStale(await readState(), {
        target: valueFromInput(input.target) ?? requestUrl.searchParams.get('target') ?? 'app',
        reason: valueFromInput(input.reason) ?? requestUrl.searchParams.get('reason') ?? undefined,
      });
      await writeState(nextState);
      writeJson(response, 200, nextState);
      return;
    }

    if (request.method === 'POST' && requestUrl.pathname === '/clear-stale') {
      const nextState = freshState();
      await writeState(nextState);
      writeJson(response, 200, nextState);
      return;
    }

    writeJson(response, 404, { error: 'Not found' });
  } catch (error) {
    writeJson(response, 500, { error: error instanceof Error ? error.message : String(error) });
  }
});

server.listen(port, host, () => {
  console.log(`Runtime status server listening at http://${host}:${port}`);
  console.log(`State file: ${stateFile}`);
});

async function ensureStateFile() {
  await mkdir(path.dirname(stateFile), { recursive: true });

  try {
    await readFile(stateFile, 'utf8');
  } catch {
    await writeState(freshState());
  }
}

async function readState() {
  try {
    return normalizeState(JSON.parse(await readFile(stateFile, 'utf8')));
  } catch {
    const nextState = freshState();
    await writeState(nextState);
    return nextState;
  }
}

async function writeState(state) {
  await mkdir(path.dirname(stateFile), { recursive: true });
  await writeFile(stateFile, `${JSON.stringify(state, null, 2)}\n`, 'utf8');
}

function freshState() {
  return {
    statusVersion: 1,
    owner,
    stale: false,
    staleTargets: [],
    generation: new Date().toISOString(),
    serverStartedAt,
  };
}

function markStale(state, input) {
  const staleTargets = normalizeTargets(input.target);

  return {
    ...state,
    stale: true,
    staleTargets,
    ...(input.reason?.trim() ? { reason: input.reason.trim() } : {}),
    generation: new Date().toISOString(),
    markedAt: new Date().toISOString(),
    serverStartedAt,
  };
}

function normalizeState(value) {
  return {
    statusVersion: 1,
    owner,
    stale: value?.stale === true,
    staleTargets: Array.isArray(value?.staleTargets) ? normalizeTargets(value.staleTargets) : [],
    ...(typeof value?.reason === 'string' && value.reason.trim()
      ? { reason: value.reason.trim() }
      : {}),
    ...(typeof value?.generation === 'string' ? { generation: value.generation } : {}),
    ...(typeof value?.markedAt === 'string' ? { markedAt: value.markedAt } : {}),
    serverStartedAt,
  };
}

function runtimeOwner() {
  return {
    instanceId: process.env.RUNTIME_INSTANCE_ID ?? 'legacy-dev',
    sessionId: process.env.RUNTIME_SESSION_ID ?? 'unassigned',
    worktreePath: process.env.RUNTIME_WORKTREE_PATH ?? process.cwd(),
    gitCommit: process.env.RUNTIME_GIT_COMMIT ?? 'unknown',
  };
}

function normalizeTargets(value) {
  const values = Array.isArray(value) ? value : String(value).split(',');
  const targets = values
    .map((entry) => String(entry).trim())
    .filter((entry) => entry === 'app' || entry === 'frontend' || entry === 'backend');

  return targets.length > 0 ? [...new Set(targets)] : ['app'];
}

function valueFromInput(value) {
  return typeof value === 'string' && value.trim() ? value.trim() : undefined;
}

async function readJsonBody(request) {
  const chunks = [];

  for await (const chunk of request) {
    chunks.push(chunk);
  }

  const body = Buffer.concat(chunks).toString('utf8').trim();

  if (!body) {
    return {};
  }

  return JSON.parse(body);
}

function writeJson(response, statusCode, body) {
  response.writeHead(statusCode, { 'Content-Type': 'application/json; charset=utf-8' });
  response.end(`${JSON.stringify(body)}\n`);
}
