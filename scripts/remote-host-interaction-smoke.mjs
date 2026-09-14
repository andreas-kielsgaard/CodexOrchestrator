// Opt-in live Codex host proof: one narrowly scoped approval and cancellation of sleep.
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { createInterface } from 'node:readline';
import { appendFileSync, mkdirSync, writeFileSync } from 'node:fs';

const target = process.env.ORCHID_SSH_TARGET ?? 'orchid-remote';
const executable = process.env.ORCHID_HOST_EXECUTABLE ?? '/root/.local/bin/orchid-host';
const configurationRef = process.env.ORCHID_REMOTE_CONFIGURATION ?? 'codex-default';
const worktree =
  process.env.ORCHID_REMOTE_WORKTREE ??
  '/root/.codex-orchestrator/repositories/orchid/worktrees/remote-development-demo';
const model = process.env.ORCHID_REMOTE_MODEL ?? 'gpt-5.6-terra';
const runId = randomUUID();
mkdirSync('.dev', { recursive: true });
const artifact = `.dev/remote-host-interactions-${runId}`;
const records = { runId, target, worktree, model, startedAt: new Date().toISOString() };
const child = spawn('ssh', ['-T', '-o', 'BatchMode=yes', target, executable, 'connect'], {
  stdio: ['pipe', 'pipe', 'pipe'],
});
const pending = new Map();
const turns = new Map();
let diagnostics = '';
let sequence = 0;
child.stderr.on('data', (chunk) => {
  diagnostics += chunk;
});

function call(method, params) {
  const id = String(++sequence);
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`${method} timed out: ${diagnostics}`));
    }, 120_000);
    pending.set(id, {
      resolve: (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      reject: (error) => {
        clearTimeout(timer);
        reject(error);
      },
    });
    child.stdin.write(`${JSON.stringify({ id, method, params })}\n`);
  });
}

const lines = createInterface({ input: child.stdout });
lines.on('line', (line) => {
  appendFileSync(`${artifact}.jsonl`, `${line}\n`);
  const frame = JSON.parse(line);
  if (frame.kind === 'response') {
    const reply = pending.get(frame.id);
    pending.delete(frame.id);
    if (frame.error) reply?.reject(new Error(JSON.stringify(frame.error)));
    else reply?.resolve(frame.result);
    return;
  }
  const turn = turns.get(frame.invocationId);
  if (!turn) return;
  if (frame.update.kind === 'finished') {
    turn.finish(frame.update.payload);
    return;
  }
  const event = frame.update.payload;
  if (event.normalized?.externalContextId)
    turn.providerContext = event.normalized.externalContextId;
  if (event.normalized?.kind === 'agent_message') turn.messages.push(event.normalized.text);
  turn.onEvent?.(event, turn).catch(turn.fail);
});
child.on('exit', (code) => {
  const error = new Error(`Host exited (${code}): ${diagnostics}`);
  for (const reply of pending.values()) reply.reject(error);
  for (const turn of turns.values()) turn.fail(error);
});

async function invoke(text, sandbox, onEvent) {
  const sessionId = `interaction-smoke-${randomUUID()}`;
  const invocationId = `interaction-smoke-${randomUUID()}`;
  let turn;
  const completion = new Promise((resolve, reject) => {
    const timeout = setTimeout(() => reject(new Error(`Turn timeout: ${invocationId}`)), 180_000);
    turn = {
      sessionId,
      invocationId,
      messages: [],
      onEvent,
      providerContext: null,
      finish: (outcome) => {
        clearTimeout(timeout);
        turns.delete(invocationId);
        resolve({
          sessionId,
          invocationId,
          providerContext: turn.providerContext,
          messages: turn.messages,
          outcome,
        });
      },
      fail: (error) => {
        clearTimeout(timeout);
        turns.delete(invocationId);
        reject(error);
      },
    };
    turns.set(invocationId, turn);
  });
  completion.catch(() => {});
  await call('invoke', {
    configurationRef,
    externalContextId: null,
    request: {
      sessionId,
      invocationId,
      submittedText: text,
      workingDirectory: worktree,
      options: { model, sandbox },
      launchExtension: {
        managedMcpServers: [],
        reasoningMode: null,
        ignoreUserRules: false,
        skillRoots: [],
        configOverrides: ['approval_policy="on-request"'],
        environment: [],
        initialPromptPrefix: null,
      },
    },
  });
  return completion;
}

try {
  records.host = await call('describe');
  const marker = `orchid-approval-smoke-${randomUUID()}`;
  const markerPath = `/tmp/${marker}.txt`;
  const command = `printf '%s' '${marker}' > '${markerPath}'`;
  let approved = null;
  const approvalTurn = await invoke(
    `This is a one-command approval integration test. Use the command execution tool to run exactly this shell command: ${command}\nRequest sandbox_permissions="require_escalated" with a justification asking approval for this single marker file. Do not request a prefix rule or any persistent permission. The client will approve this exact command once. Do not use apply_patch or any alternative command. After it succeeds, reply APPROVAL-SMOKE-PASS.`,
    'read_only',
    async (event, turn) => {
      if (event.rawPayload?.kind !== 'runtime_request_opened') return;
      const request = event.rawPayload.request;
      console.log('Approval request', JSON.stringify(request));
      const observedCommand =
        typeof request.command === 'string' ? request.command : JSON.stringify(request.command);
      assert.equal(request.kind, 'approval');
      assert.ok(!approved, 'Only one approval should be requested');
      assert.ok(
        observedCommand?.includes(command),
        `Unexpected requested command: ${observedCommand}`,
      );
      const choice = request.choices.find((item) => item.response?.decision === 'accept');
      assert.ok(choice, 'A one-time approval choice must be offered');
      approved = {
        requestId: request.id,
        command: request.command,
        response: choice.response,
        at: new Date().toISOString(),
      };
      await call('respond', {
        invocationId: turn.invocationId,
        requestId: request.id,
        response: choice.response,
      });
    },
  );
  assert.ok(approved, 'Provider must request approval');
  assert.equal(approvalTurn.outcome.status, 'completed', JSON.stringify(approvalTurn.outcome));
  const observedMarker = execFileSync(
    'ssh',
    ['-T', '-o', 'BatchMode=yes', target, `cat '${markerPath}'`],
    { encoding: 'utf8' },
  );
  assert.equal(observedMarker, marker);
  records.approval = {
    ...approvalTurn,
    approval: approved,
    markerPath,
    independentlyObserved: observedMarker,
  };
  console.log('Approval completed', JSON.stringify(records.approval));

  let canceled = null;
  const cancelTurn = await invoke(
    'This is a cancellation integration test. Run exactly `sleep 60` with the command execution tool, wait for it to finish, then say SLEEP-FINISHED. Do not perform other actions. The client will interrupt the running command.',
    'read_only',
    async (event, turn) => {
      const raw = event.rawPayload;
      if (canceled || raw?.type !== 'item.started' || raw.item?.type !== 'command_execution')
        return;
      assert.ok(JSON.stringify(raw.item.command).includes('sleep 60'), 'Expected sleep command');
      canceled = {
        at: new Date().toISOString(),
        command: raw.item.command,
        nativeItemId: raw.item.id,
      };
      await call('cancel', { invocationId: turn.invocationId });
    },
  );
  assert.ok(canceled, 'Cancel must happen after a command starts');
  assert.equal(cancelTurn.outcome.status, 'canceled', JSON.stringify(cancelTurn.outcome));
  records.cancellation = { ...cancelTurn, cancellation: canceled };
  records.completedAt = new Date().toISOString();
  console.log('Cancellation completed', JSON.stringify(records.cancellation));
} catch (error) {
  records.error = error.stack;
  throw error;
} finally {
  writeFileSync(`${artifact}.json`, JSON.stringify(records, null, 2));
  console.log('Evidence', `${artifact}.json`);
  child.stdin.end();
  setTimeout(() => child.kill(), 5_000).unref();
}
