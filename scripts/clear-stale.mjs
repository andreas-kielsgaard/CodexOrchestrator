import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

const stateFile =
  process.env.RUNTIME_STATUS_FILE ?? path.join(process.cwd(), '.dev', 'runtime-status.json');
const statusUrl = process.env.RUNTIME_STATUS_URL ?? 'http://127.0.0.1:41415/clear-stale';
const nextState = {
  statusVersion: 1,
  stale: false,
  staleTargets: [],
  generation: new Date().toISOString(),
};

if (!(await postClear())) {
  await mkdir(path.dirname(stateFile), { recursive: true });
  await writeFile(stateFile, `${JSON.stringify(nextState, null, 2)}\n`, 'utf8');
}

console.log('Cleared runtime stale status');

async function postClear() {
  try {
    const response = await fetch(statusUrl, { method: 'POST' });
    return response.ok;
  } catch {
    return false;
  }
}
