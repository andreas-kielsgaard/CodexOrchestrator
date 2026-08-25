#!/usr/bin/env node

import { execFile } from 'node:child_process';
import { mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { DatabaseSync } from 'node:sqlite';
import { promisify } from 'node:util';
import { assertOwnedDebugger, loopbackUrl, resolveTarget } from './webview-control.mjs';

const execFileAsync = promisify(execFile);
const workspace = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const acceptedCommit = '518a802a3c21c56bfaa5d9aba0052958271a4b0d';
const profileId = 'native-profile-ed6f9ea9-f8a8-411b-ba0b-490859dc6126';
const command = 'probe_native_profile_mcp_reporting';
const debugUrl = loopbackUrl('http://127.0.0.1:9223');
const targetUrl = 'http://127.0.0.1:1880/';
const manifestPath = path.join(workspace, '.dev', 'worktree-runtime', 'ps2-live-20260808', 'manifest.json');

export function fixedInvokeExpression() {
  return `(async () => { const invoke = globalThis.__TAURI_INTERNALS__?.invoke; if (typeof invoke !== 'function') return { disposition: 'tauri_internals_unavailable' }; try { await invoke('${command}', { input: { profileId: '${profileId}' } }); return { disposition: 'fulfilled' }; } catch (_) { return { disposition: 'rejected' }; } })()`;
}

export function classifyProbe(rows) {
  if (!Array.isArray(rows) || rows.length === 0) return { disposition: 'absent' };
  const latest = rows[0];
  return {
    disposition: ['pending', 'dispatching'].includes(latest.state) ? 'current' : 'not_current',
    state: latest.state,
    requestedAt: latest.requestedAt,
    deadlineAt: latest.deadlineAt,
  };
}

async function main() {
  if (process.argv.length !== 2) throw new Error('This dispatcher accepts no arguments.');
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
  assertManifest(manifest);
  const executablePath = path.join(manifest.projected.paths.cargoTarget, 'debug', 'codex-orchestrator.exe');
  const pid = await exactRuntimePid(executablePath);
  const ownership = await assertOwnedDebugger({ executablePath, pid, debugPort: 9223 });
  const preDispatchProbe = observeProbe(manifest.projected.paths.appData);
  if (preDispatchProbe.disposition === 'current') {
    throw new Error('A current application-owned MCP probe already exists; refusing duplicate dispatch.');
  }
  const target = await resolveTarget(debugUrl, targetUrl);
  const commandDispatch = await dispatchFixedCommand(target.webSocketDebuggerUrl);
  const durableProbe = observeProbe(manifest.projected.paths.appData);
  const receipt = {
    schemaVersion: 'native-mcp-probe-dispatch/v1',
    observedAt: new Date().toISOString(),
    binding: {
      runtimeInstance: manifest.identity.instanceId,
      runtimeCommit: manifest.identity.gitCommit,
      executablePath,
      pid,
      profileId,
      command,
      targetUrl,
    },
    ownership,
    preDispatchProbe,
    commandDispatch,
    durableProbe,
    boundaries: {
      dispatch: 'Fixed Tauri command dispatch does not prove durable persistence, MCP exchange, receipt, readiness, provider activity, or workflow completion.',
      durableProbe: 'The probe observation is a read-only SQLite query; it does not create, alter, settle, or accept the probe.',
    },
  };
  const out = path.join(manifest.projected.paths.root, 'review-evidence', 'native-mcp-probe-dispatch.json');
  await writeReceipt(out, receipt);
  process.stdout.write(`${JSON.stringify(receipt, null, 2)}\n`);
}

function assertManifest(manifest) {
  if (
    manifest?.identity?.instanceId !== 'ps2-live-20260808' ||
    manifest?.identity?.gitCommit !== acceptedCommit ||
    !samePath(manifest?.identity?.worktreePath, workspace)
  ) {
    throw new Error('The retained runtime manifest does not match this accepted PS-2 route.');
  }
  if (!manifest?.projected?.paths?.cargoTarget || !manifest?.projected?.paths?.appData) {
    throw new Error('The retained runtime manifest lacks required app paths.');
  }
}

function samePath(left, right) {
  return typeof left === 'string' && path.resolve(left).toLowerCase() === path.resolve(right).toLowerCase();
}

async function exactRuntimePid(executablePath) {
  const query = 'Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq $env:NATIVE_MCP_PROBE_EXE } | Select-Object -ExpandProperty ProcessId | ConvertTo-Json -Compress';
  const { stdout } = await execFileAsync('powershell.exe', ['-NoProfile', '-Command', query], {
    encoding: 'utf8',
    windowsHide: true,
    timeout: 15_000,
    env: { ...process.env, NATIVE_MCP_PROBE_EXE: executablePath },
  });
  const values = JSON.parse(stdout.trim() || '[]');
  const pids = (Array.isArray(values) ? values : [values]).filter((value) => Number.isSafeInteger(value) && value > 0);
  if (pids.length !== 1) throw new Error(`Expected exactly one retained runtime executable; observed ${pids.length}.`);
  return pids[0];
}

function observeProbe(appData) {
  const databasePath = path.join(appData, 'codex-orchestrator-active-v3.sqlite');
  const database = new DatabaseSync(databasePath, { readOnly: true });
  try {
    database.exec('PRAGMA query_only=ON');
    const rows = database
      .prepare("SELECT state,requested_at AS requestedAt,deadline_at AS deadlineAt FROM native_codex_profile_mcp_probes WHERE profile_id=? ORDER BY requested_at DESC,request_id DESC LIMIT 1")
      .all(profileId);
    return classifyProbe(rows);
  } finally {
    database.close();
  }
}

async function dispatchFixedCommand(webSocketUrl) {
  const socket = new WebSocket(webSocketUrl);
  await once(socket, 'open');
  try {
    const result = await request(socket, {
      id: 1,
      method: 'Runtime.evaluate',
      params: { expression: fixedInvokeExpression(), awaitPromise: true, returnByValue: true, silent: true },
    });
    const disposition = result.result?.value?.disposition;
    return ['fulfilled', 'rejected', 'tauri_internals_unavailable'].includes(disposition)
      ? { disposition }
      : { disposition: 'protocol_result_unrecognized' };
  } finally {
    socket.close();
  }
}

function request(socket, payload) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Fixed Tauri command dispatch timed out.')), 15_000);
    socket.addEventListener('message', (event) => {
      try {
        const message = JSON.parse(String(event.data));
        if (message.id !== payload.id) return;
        clearTimeout(timer);
        if (message.error) reject(new Error(`Fixed Tauri command dispatch failed: ${message.error.message}`));
        else resolve(message.result ?? {});
      } catch (error) { clearTimeout(timer); reject(error); }
    });
    socket.addEventListener('error', () => { clearTimeout(timer); reject(new Error('Fixed Tauri command socket failed.')); }, { once: true });
    socket.send(JSON.stringify(payload));
  });
}

function once(socket, eventName) {
  return new Promise((resolve, reject) => {
    socket.addEventListener(eventName, resolve, { once: true });
    socket.addEventListener('error', () => reject(new Error('Fixed Tauri command socket could not open.')), { once: true });
  });
}

async function writeReceipt(out, receipt) {
  await mkdir(path.dirname(out), { recursive: true });
  const temporary = `${out}.tmp`;
  await writeFile(temporary, `${JSON.stringify(receipt, null, 2)}\n`, 'utf8');
  await rename(temporary, out);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    process.stderr.write(`native-mcp-probe-dispatch: ${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  });
}
