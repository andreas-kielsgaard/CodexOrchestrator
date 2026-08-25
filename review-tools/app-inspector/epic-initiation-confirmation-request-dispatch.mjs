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
const runtimeCommit = '518a802a3c21c56bfaa5d9aba0052958271a4b0d';
const draftId = 'epic-planning-draft-ff5cdb5043454f168b6079f92f47a164';
const sessionId = 'be35756b-3a9d-4f99-ae86-1ebf19e00b9d';
const command = 'request_epic_initiation_confirmation';
const debugUrl = loopbackUrl('http://127.0.0.1:9223');
const targetUrl = 'http://127.0.0.1:1880/';
const manifestPath = path.join(workspace, '.dev', 'worktree-runtime', 'ps2-live-20260808', 'manifest.json');

export function deriveRequestBinding({ draft, proposal, sessionAssociation, initiationCount }) {
  if (draft?.id !== draftId || draft?.status !== 'active') throw new Error('The retained planning draft is not active.');
  if (proposal?.draftId !== draftId || !proposal.id || !proposal.revisionToken) {
    throw new Error('The retained draft has no current proposal binding.');
  }
  if (sessionAssociation?.draftId !== draftId || sessionAssociation?.sessionId !== sessionId) {
    throw new Error('The retained Plan Builder Session is not associated with this draft.');
  }
  if (initiationCount !== 0) throw new Error('An Epic initiation already exists; refusing a new confirmation request.');
  return {
    epicPlanningDraftId: draftId,
    proposalRevisionId: proposal.id,
    expectedRevisionToken: proposal.revisionToken,
    idempotencyKey: `initiate:${draftId}:${proposal.id}`,
  };
}

export function fixedInvokeExpression(binding) {
  return `(async () => { const invoke = globalThis.__TAURI_INTERNALS__?.invoke; if (typeof invoke !== 'function') return { disposition: 'tauri_internals_unavailable' }; try { const result = await invoke('${command}', { input: { epicPlanningDraftId: '${binding.epicPlanningDraftId}', expectedRevisionToken: '${binding.expectedRevisionToken}', idempotencyKey: '${binding.idempotencyKey}' } }); return { disposition: 'fulfilled', requestId: result?.requestId ?? null, requestDraftId: result?.epicPlanningDraftId ?? null, state: result?.state ?? null, sourceKind: result?.source?.kind ?? null }; } catch (_) { return { disposition: 'rejected' }; } })()`;
}

async function main() {
  if (process.argv.length !== 2) throw new Error('This dispatcher accepts no arguments.');
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
  assertManifest(manifest);
  const executablePath = path.join(manifest.projected.paths.cargoTarget, 'debug', 'codex-orchestrator.exe');
  const pid = await exactRuntimePid(executablePath);
  const ownership = await assertOwnedDebugger({ executablePath, pid, debugPort: 9223 });
  const preDispatch = observeBinding(manifest.projected.paths.appData);
  const binding = deriveRequestBinding(preDispatch);
  const target = await resolveTarget(debugUrl, targetUrl);
  const commandResult = await dispatchFixedCommand(target.webSocketDebuggerUrl, binding);
  const durableConfirmationRequest = observeConfirmationDurability(manifest.projected.paths.appData);
  const postDispatch = observeBinding(manifest.projected.paths.appData);
  const receipt = {
    schemaVersion: 'epic-initiation-confirmation-request-dispatch/v1',
    observedAt: new Date().toISOString(),
    binding: {
      runtimeInstance: manifest.identity.instanceId,
      runtimeCommit: manifest.identity.gitCommit,
      executablePath,
      pid,
      sessionId,
      command,
      targetUrl,
      ...binding,
    },
    ownership,
    preDispatch,
    commandResult,
    durableConfirmationRequest,
    postDispatch,
    boundaries: {
      command: 'The fixed command can only request/open the application confirmation. It cannot provide a root branch, confirm, initiate directly, or mutate SQLite.',
      confirmationPersistence: 'The retained product has no durable confirmation-request table. A command result and later visible modal must be observed separately; neither proves confirmed initiation.',
      durableRead: 'All database observations are SQLite read-only with PRAGMA query_only=ON.',
    },
  };
  const out = path.join(manifest.projected.paths.root, 'review-evidence', 'epic-initiation-confirmation-request-dispatch.json');
  await writeReceipt(out, receipt);
  process.stdout.write(`${JSON.stringify(receipt, null, 2)}\n`);
}

function assertManifest(manifest) {
  if (
    manifest?.identity?.instanceId !== 'ps2-live-20260808' ||
    manifest?.identity?.gitCommit !== runtimeCommit ||
    !samePath(manifest?.identity?.worktreePath, workspace)
  ) throw new Error('The retained runtime manifest does not match this accepted PS-2 route.');
  if (!manifest?.projected?.paths?.cargoTarget || !manifest?.projected?.paths?.appData) {
    throw new Error('The retained runtime manifest lacks required app paths.');
  }
}

function samePath(left, right) {
  return typeof left === 'string' && path.resolve(left).toLowerCase() === path.resolve(right).toLowerCase();
}

async function exactRuntimePid(executablePath) {
  const query = 'Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq $env:EPIC_CONFIRMATION_REQUEST_EXE } | Select-Object -ExpandProperty ProcessId | ConvertTo-Json -Compress';
  const { stdout } = await execFileAsync('powershell.exe', ['-NoProfile', '-Command', query], {
    encoding: 'utf8', windowsHide: true, timeout: 15_000,
    env: { ...process.env, EPIC_CONFIRMATION_REQUEST_EXE: executablePath },
  });
  const values = JSON.parse(stdout.trim() || '[]');
  const pids = (Array.isArray(values) ? values : [values]).filter((value) => Number.isSafeInteger(value) && value > 0);
  if (pids.length !== 1) throw new Error(`Expected exactly one retained runtime executable; observed ${pids.length}.`);
  return pids[0];
}

function observeBinding(appData) {
  const database = new DatabaseSync(path.join(appData, 'codex-orchestrator-active-v3.sqlite'), { readOnly: true });
  try {
    database.exec('PRAGMA query_only=ON');
    const draft = database.prepare('SELECT id,status FROM epic_planning_drafts WHERE id=?').get(draftId);
    const proposal = database.prepare('SELECT id,draft_id AS draftId,revision_token AS revisionToken FROM proposal_revisions WHERE draft_id=? ORDER BY recorded_at DESC,id DESC LIMIT 1').get(draftId);
    const sessionAssociation = database.prepare("SELECT draft_id AS draftId,agent_session_id AS sessionId FROM planning_draft_agent_session_associations WHERE draft_id=? AND agent_session_id=? AND actor_id='managed-plan-builder' ORDER BY associated_at,id LIMIT 1").get(draftId, sessionId);
    const initiationCount = database.prepare('SELECT COUNT(*) AS count FROM epic_initiations WHERE draft_id=?').get(draftId).count;
    return { draft, proposal, sessionAssociation, initiationCount };
  } finally { database.close(); }
}

function observeConfirmationDurability(appData) {
  const database = new DatabaseSync(path.join(appData, 'codex-orchestrator-active-v3.sqlite'), { readOnly: true });
  try {
    database.exec('PRAGMA query_only=ON');
    const table = database.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='epic_initiation_confirmation_requests'").get();
    return table ? { disposition: 'unexpected_durable_table_present' } : { disposition: 'not_product_durable' };
  } finally { database.close(); }
}

async function dispatchFixedCommand(webSocketUrl, binding) {
  const socket = new WebSocket(webSocketUrl);
  await once(socket, 'open');
  try {
    const result = await request(socket, {
      id: 1, method: 'Runtime.evaluate',
      params: { expression: fixedInvokeExpression(binding), awaitPromise: true, returnByValue: true, silent: true },
    });
    const value = result.result?.value;
    if (!value || !['fulfilled', 'rejected', 'tauri_internals_unavailable'].includes(value.disposition)) {
      return { disposition: 'protocol_result_unrecognized' };
    }
    return {
      disposition: value.disposition,
      ...(value.disposition === 'fulfilled' ? {
        requestId: typeof value.requestId === 'string' ? value.requestId : null,
        requestDraftId: typeof value.requestDraftId === 'string' ? value.requestDraftId : null,
        state: typeof value.state === 'string' ? value.state : null,
        sourceKind: typeof value.sourceKind === 'string' ? value.sourceKind : null,
      } : {}),
    };
  } finally { socket.close(); }
}

function request(socket, payload) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Fixed confirmation request dispatch timed out.')), 15_000);
    socket.addEventListener('message', (event) => {
      try {
        const message = JSON.parse(String(event.data));
        if (message.id !== payload.id) return;
        clearTimeout(timer);
        if (message.error) reject(new Error(`Fixed confirmation request dispatch failed: ${message.error.message}`));
        else resolve(message.result ?? {});
      } catch (error) { clearTimeout(timer); reject(error); }
    });
    socket.addEventListener('error', () => { clearTimeout(timer); reject(new Error('Fixed confirmation request socket failed.')); }, { once: true });
    socket.send(JSON.stringify(payload));
  });
}

function once(socket, eventName) {
  return new Promise((resolve, reject) => {
    socket.addEventListener(eventName, resolve, { once: true });
    socket.addEventListener('error', () => reject(new Error('Fixed confirmation request socket could not open.')), { once: true });
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
    process.stderr.write(`epic-initiation-confirmation-request-dispatch: ${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  });
}
