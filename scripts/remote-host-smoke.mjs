// Live test against the configured demo worktree. Writes one identified marker file.
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { createInterface } from 'node:readline';
import { randomUUID } from 'node:crypto';

const target = process.env.ORCHID_SSH_TARGET ?? 'orchid-remote';
const executable = process.env.ORCHID_HOST_EXECUTABLE ?? '/root/.local/bin/orchid-host';
const repository =
  process.env.ORCHID_REMOTE_REPOSITORY ?? '/root/.codex-orchestrator/repositories/orchid/source';
const branch = process.env.ORCHID_DEMO_BRANCH ?? 'refs/heads/codex/remote-development-demo';
const configurationRef = process.env.ORCHID_REMOTE_CONFIGURATION ?? 'codex-default';
const child = spawn('ssh', ['-T', '-o', 'BatchMode=yes', target, executable, 'connect'], {
  stdio: ['pipe', 'pipe', 'pipe'],
});
const replies = new Map();
const turns = new Map();
let requestNumber = 0;
let demoWorktree;
let diagnostic = '';
child.stderr.on('data', (chunk) => {
  diagnostic += chunk;
});
function call(method, params) {
  const id = String(++requestNumber);
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      replies.delete(id);
      reject(new Error(`${method} timed out: ${diagnostic}`));
    }, 120_000);
    replies.set(id, {
      resolve: (value) => {
        clearTimeout(timeout);
        resolve(value);
      },
      reject: (error) => {
        clearTimeout(timeout);
        reject(error);
      },
    });
    child.stdin.write(`${JSON.stringify({ id, method, params })}\n`);
  });
}
const lines = createInterface({ input: child.stdout });
lines.on('line', (line) => {
  const frame = JSON.parse(line);
  if (frame.kind === 'response') {
    const reply = replies.get(frame.id);
    replies.delete(frame.id);
    if (frame.error) reply?.reject(new Error(JSON.stringify(frame.error)));
    else reply?.resolve(frame.result);
    return;
  }
  const turn = turns.get(frame.invocationId);
  if (!turn) return;
  const update = frame.update;
  if (update.kind === 'finished') {
    turn.finish(update.payload);
    return;
  }
  const event = update.payload;
  const normalized = event.normalized;
  if (normalized?.externalContextId) turn.externalContextId = normalized.externalContextId;
  if (normalized?.kind === 'agent_message') turn.messages.push(normalized.text);
  if (normalized?.kind === 'tool_activity') turn.toolEvents++;
});
child.on('exit', (code) => {
  const error = new Error(`SSH host exited (${code}): ${diagnostic}`);
  for (const reply of replies.values()) reply.reject(error);
  for (const turn of turns.values()) turn.fail(error);
});
async function invoke(sessionId, text, model, externalContextId) {
  const invocationId = `host-smoke-${randomUUID()}`;
  let turn;
  const completed = new Promise((resolve, reject) => {
    const timeout = setTimeout(
      () => reject(new Error(`Invocation timed out: ${invocationId}`)),
      240_000,
    );
    turn = {
      messages: [],
      toolEvents: 0,
      externalContextId: null,
      finish: (outcome) => {
        clearTimeout(timeout);
        turns.delete(invocationId);
        resolve({ ...turn, outcome });
      },
      fail: (error) => {
        clearTimeout(timeout);
        turns.delete(invocationId);
        reject(error);
      },
    };
    turns.set(invocationId, turn);
  });
  completed.catch(() => {});
  await call('invoke', {
    configurationRef,
    externalContextId: externalContextId ?? null,
    request: {
      sessionId,
      invocationId,
      submittedText: text,
      workingDirectory: demoWorktree,
      options: { model, sandbox: 'workspace_write' },
      launchExtension: null,
    },
  });
  return completed;
}
try {
  console.log('Host', await call('describe'));
  const worktrees = await call('list_worktrees', { repositoryRoot: repository, branchRef: branch });
  assert.equal(worktrees.length, 1, 'Expected the manually prepared branch worktree');
  demoWorktree = worktrees[0].path;
  console.log('Worktree', worktrees[0]);
  if (!process.argv.includes('--discover-only')) {
    const capabilities = await call('capabilities', {
      configurationRef,
      workingDirectory: demoWorktree,
    });
    const models = capabilities.profile.exposure.models;
    const model = models.includes('gpt-5.6-terra') ? 'gpt-5.6-terra' : models[0];
    assert.ok(model, 'No models discovered on the remote configuration');
    const sessionId = `host-smoke-${randomUUID()}`;
    const marker = `orchid-host-smoke-${randomUUID()}`;
    const file = `${marker}.txt`;
    const first = await invoke(
      sessionId,
      `Report your actual current working directory, read the first line of README.md, and create ${file} in the current directory containing exactly ${marker}. Do not change other files.`,
      model,
    );
    assert.equal(first.outcome.status, 'completed', JSON.stringify(first.outcome));
    assert.ok(first.externalContextId);
    console.log('First turn', {
      providerContext: first.externalContextId,
      messages: first.messages,
    });
    const observed = execFileSync(
      'ssh',
      ['-T', '-o', 'BatchMode=yes', target, `cat '${demoWorktree}/${file}'`],
      { encoding: 'utf8' },
    ).trim();
    assert.equal(observed, marker);
    const second = await invoke(
      sessionId,
      `Read ${file} and reply with its exact contents and your current working directory. Do not modify files.`,
      model,
      first.externalContextId,
    );
    assert.equal(second.outcome.status, 'completed', JSON.stringify(second.outcome));
    assert.equal(second.externalContextId, first.externalContextId);
    assert.ok(second.messages.join('\n').includes(marker));
    console.log(
      JSON.stringify(
        {
          sessionId,
          providerContext: first.externalContextId,
          model,
          worktree: demoWorktree,
          markerFile: file,
          independentlyObserved: observed,
          followup: second.messages,
        },
        null,
        2,
      ),
    );
  }
} finally {
  child.stdin.end();
  setTimeout(() => child.kill(), 5_000).unref();
}
