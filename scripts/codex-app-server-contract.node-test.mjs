// Opt-in executable contract tests. The real CLI talks only to a local fixture provider,
// using a disposable CODEX_HOME with no copied authentication or user configuration.
import assert from 'node:assert/strict';
import { execFile, spawn } from 'node:child_process';
import { EventEmitter, once } from 'node:events';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { createInterface } from 'node:readline';
import test from 'node:test';
import { promisify } from 'node:util';

const executable = process.env.CODEX_APP_SERVER_CONTRACT_PROGRAM;

test(
  'installed app-server: independent history fork and continuation',
  {
    skip: !executable,
    timeout: 90_000,
  },
  async (t) => {
    const root = await mkdtemp(path.join(tmpdir(), 'orchid-import-contract-'));
    const home = path.join(root, 'home');
    const cwd = path.join(root, 'workspace');
    await mkdir(home);
    await mkdir(cwd);
    const provider = await fixtureProvider(t);
    await writeFile(
      path.join(home, 'config.toml'),
      configuration(provider.port, 'gpt-5.6-terra', 'low'),
    );
    const connections = [];
    const connect = async () => {
      const server = new AppServer(home, cwd);
      connections.push(server);
      await server.start();
      return server;
    };
    t.after(async () => {
      for (const server of connections) await server.close();
      assert.equal(path.resolve(path.dirname(root)), path.resolve(tmpdir()));
      assert.ok(path.basename(root).startsWith('orchid-import-contract-'));
      await rm(root, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
    });
    let server = await connect();
    const source = (await server.call('thread/start', { cwd })).thread.id;
    const turn = (
      await server.call('turn/start', {
        threadId: source,
        input: [{ type: 'text', text: 'Remember IMPORT_CONTEXT_MARKER.' }],
      })
    ).turn.id;
    await server.waitFor((m) => m.method === 'turn/completed' && m.params.turn.id === turn);
    await server.close();
    server = await connect();
    const before = (await server.call('thread/read', { threadId: source, includeTurns: true }))
      .thread;
    assert.equal(before.turns.length, 1);
    const fork = (await server.call('thread/fork', { threadId: source, lastTurnId: turn, cwd }))
      .thread;
    assert.notEqual(fork.id, source);
    assert.deepEqual(fork.turns, before.turns);
    if (process.env.ORCHID_IMPORT_FIXTURE_OUT) {
      await writeFile(
        process.env.ORCHID_IMPORT_FIXTURE_OUT,
        JSON.stringify({ thread: before }, null, 2),
      );
    }
    await server.close();
    server = await connect();
    await server.call('thread/resume', { threadId: fork.id, cwd });
    const next = (
      await server.call('turn/start', {
        threadId: fork.id,
        input: [{ type: 'text', text: 'Continue with the remembered context.' }],
      })
    ).turn.id;
    await server.waitFor((m) => m.method === 'turn/completed' && m.params.turn.id === next);
    assert.ok(JSON.stringify(provider.requests.at(-1).input).includes('IMPORT_CONTEXT_MARKER'));
    assert.equal(
      (await server.call('thread/read', { threadId: source, includeTurns: true })).thread.turns
        .length,
      1,
    );
    if (process.env.ORCHID_IMPORT_TEST_BINARY) {
      await server.close();
      const { stdout } = await promisify(execFile)(
        process.env.ORCHID_IMPORT_TEST_BINARY,
        [
          'installed_codex_import_reopens_and_continues_through_orchid',
          '--ignored',
          '--nocapture',
          '--test-threads=1',
        ],
        {
          windowsHide: true,
          timeout: 60_000,
          env: {
            ...process.env,
            ORCHID_IMPORT_CONTRACT_HOME: home,
            ORCHID_IMPORT_CONTRACT_ROOT: root,
            ORCHID_IMPORT_CONTRACT_THREAD: source,
          },
        },
      );
      assert.match(stdout, /1 passed/);
      const imported = JSON.parse(
        await readFile(path.join(root, 'orchid-import-result.json'), 'utf8'),
      );
      assert.notEqual(imported.forkId, source);
      assert.ok(JSON.stringify(provider.requests.at(-1).input).includes('IMPORT_CONTEXT_MARKER'));
      t.diagnostic(
        'Orchid imported through its real adapter, reopened SQLite, and continued through its normal native-bound runtime.',
      );
    }
    t.diagnostic('Fork persisted, inherited model context, and left source history unchanged.');
  },
);
const timeout = (promise, milliseconds, description) => {
  let timer;
  return Promise.race([
    promise,
    new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error(`Timed out: ${description}`)), milliseconds);
    }),
  ]).finally(() => clearTimeout(timer));
};

class AppServer {
  constructor(home, cwd) {
    this.pending = new Map();
    this.messages = [];
    this.events = new EventEmitter();
    this.nextId = 0;
    this.stderr = '';
    const env = { ...process.env, CODEX_HOME: home };
    delete env.OPENAI_API_KEY;
    for (const key of [
      'CODEX_APP_TOOLS_PIPE_PATH',
      'CODEX_SESSION_ID',
      'CODEX_THREAD_ID',
      'CODEX_INTERNAL_ORIGINATOR_OVERRIDE',
      'CODEX_PERMISSION_PROFILE',
    ])
      delete env[key];
    this.child = spawn(executable, ['app-server'], {
      cwd,
      env,
      windowsHide: true,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    this.exited = once(this.child, 'exit');
    this.child.stderr.setEncoding('utf8').on('data', (text) => {
      this.stderr = (this.stderr + text).slice(-8000);
    });
    createInterface({ input: this.child.stdout }).on('line', (line) => {
      const message = JSON.parse(line);
      if ('method' in message) {
        this.messages.push(message);
        this.events.emit('message', message);
        if ('id' in message) {
          this.write({
            id: message.id,
            error: { code: -32601, message: 'Unsupported by the executable contract fixture' },
          });
        }
      } else {
        const pending = this.pending.get(message.id);
        this.pending.delete(message.id);
        if (message.error) pending?.reject(new Error(JSON.stringify(message.error)));
        else pending?.resolve(message.result);
      }
    });
    this.child.on('exit', () => {
      for (const pending of this.pending.values()) {
        pending.reject(new Error(`App-server exited while awaiting a reply: ${this.stderr}`));
      }
      this.pending.clear();
    });
  }

  write(message) {
    this.child.stdin.write(`${JSON.stringify(message)}\n`);
  }

  async call(method, params) {
    const id = ++this.nextId;
    const result = new Promise((resolve, reject) => this.pending.set(id, { resolve, reject }));
    this.write({ id, method, params });
    try {
      return await timeout(result, 30_000, method);
    } finally {
      this.pending.delete(id);
    }
  }

  async start() {
    await this.call('initialize', {
      clientInfo: { name: 'orchestrator_contract_test', version: '1' },
      capabilities: { experimentalApi: true },
    });
    this.write({ method: 'initialized', params: {} });
  }

  async waitFor(predicate) {
    const existing = this.messages.find(predicate);
    if (existing) return existing;
    let listener;
    const waiting = new Promise((resolve) => {
      listener = (message) => {
        if (predicate(message)) resolve(message);
      };
      this.events.on('message', listener);
    });
    try {
      return await timeout(waiting, 30_000, 'app-server notification');
    } finally {
      this.events.off('message', listener);
    }
  }

  async close() {
    try {
      if (this.child.exitCode === null && this.child.signalCode === null) {
        if (process.platform === 'win32') {
          // The fixture owns this entire child tree. Code Mode descendants can outlive EOF
          // and hold the disposable cwd open after the app-server process exits.
          const cleanup = spawn('taskkill.exe', ['/PID', String(this.child.pid), '/T', '/F'], {
            windowsHide: true,
            stdio: 'ignore',
          });
          await timeout(once(cleanup, 'exit'), 5000, 'fixture process-tree cleanup');
          await timeout(this.exited, 5000, 'app-server termination');
        } else {
          this.child.stdin.end();
          try {
            await timeout(this.exited, 8000, 'app-server EOF shutdown');
          } catch {
            this.child.kill();
            await timeout(this.exited, 5000, 'app-server termination');
          }
        }
      }
    } finally {
      this.child.stdin.destroy();
      this.child.stdout.destroy();
      this.child.stderr.destroy();
    }
  }
}

async function fixtureProvider(t) {
  const requests = [];
  const arrivals = new EventEmitter();
  const server = createServer(async (request, response) => {
    const chunks = [];
    for await (const chunk of request) chunks.push(chunk);
    const body = JSON.parse(Buffer.concat(chunks).toString('utf8'));
    requests.push(body);
    arrivals.emit('request', body);
    response.writeHead(200, { 'Content-Type': 'text/event-stream', Connection: 'close' });
    const text = JSON.stringify(body.input).includes('STEERING_MARKER') ? 'STEERED' : 'FIXTURE_OK';
    const item = {
      id: `msg_${requests.length}`,
      type: 'message',
      role: 'assistant',
      status: 'completed',
      content: [{ type: 'output_text', text, annotations: [] }],
    };
    const result = {
      id: `resp_${requests.length}`,
      object: 'response',
      status: 'completed',
      output: [item],
      usage: { input_tokens: 1, output_tokens: 1, total_tokens: 2 },
    };
    response.write(
      `data: ${JSON.stringify({ type: 'response.created', response: { id: result.id, status: 'in_progress', output: [] } })}\n\n`,
    );
    // Leave time for a steer or interrupt after the provider request has actually begun.
    const timer = setTimeout(() => {
      const events = [
        {
          type: 'response.output_item.added',
          output_index: 0,
          item: { ...item, status: 'in_progress', content: [] },
        },
        {
          type: 'response.output_text.delta',
          output_index: 0,
          content_index: 0,
          item_id: item.id,
          delta: text,
        },
        { type: 'response.output_item.done', output_index: 0, item },
        { type: 'response.completed', response: result },
      ];
      for (const event of events) response.write(`data: ${JSON.stringify(event)}\n\n`);
      response.end();
    }, 1000);
    response.on('close', () => clearTimeout(timer));
  });
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  t.after(async () => {
    const closed = new Promise((resolve) => server.close(resolve));
    server.closeAllConnections();
    await closed;
  });
  return {
    port: server.address().port,
    requests,
    async nextRequest() {
      return timeout(once(arrivals, 'request'), 20_000, 'local provider request');
    },
  };
}

const configuration = (port, model, effort, approval = 'on-request') => `
model = "${model}"
model_provider = "contract"
model_reasoning_effort = "${effort}"
approval_policy = "${approval}"
sandbox_mode = "read-only"
[model_providers.contract]
name = "Orchestrator contract fixture"
base_url = "http://127.0.0.1:${port}/v1"
wire_api = "responses"
requires_openai_auth = false
`;

test(
  'installed app-server: quick features, skill mentions, steering, interruption, resume and native defaults',
  {
    skip:
      !executable &&
      'Set CODEX_APP_SERVER_CONTRACT_PROGRAM to the installed native Codex executable',
    timeout: 120_000,
  },
  async (t) => {
    const version = await promisify(execFile)(executable, ['--version'], { windowsHide: true });
    t.diagnostic(version.stdout.trim());
    const root = await mkdtemp(path.join(tmpdir(), 'orchestrator-app-server-contract-'));
    const home = path.join(root, 'home');
    const cwd = path.join(root, 'workspace');
    await Promise.all([mkdir(home), mkdir(cwd)]);
    const extraRoot = path.join(root, 'orchestration-skills');
    const extraSkill = path.join(extraRoot, 'orchestrator-contract');
    await mkdir(extraSkill, { recursive: true });
    await writeFile(
      path.join(extraSkill, 'SKILL.md'),
      '---\nname: orchestrator-contract\ndescription: Exercise the orchestration skill root.\n---\nReply SKILL_ROOT_MARKER.\n',
    );
    const connections = [];
    t.after(async () => {
      for (const connection of connections) await connection.close();
      // This directory was allocated by this test and must stay directly inside the temp root.
      assert.equal(path.resolve(path.dirname(root)), path.resolve(tmpdir()));
      assert.ok(path.basename(root).startsWith('orchestrator-app-server-contract-'));
      await rm(root, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
    });
    const provider = await fixtureProvider(t);
    await writeFile(
      path.join(home, 'config.toml'),
      configuration(provider.port, 'gpt-5.6-terra', 'low'),
    );
    const connect = async () => {
      const connection = new AppServer(home, cwd);
      connections.push(connection);
      await connection.start();
      await connection.call('skills/extraRoots/set', { extraRoots: [extraRoot] });
      return connection;
    };

    let server = await connect();
    const skills = await server.call('skills/list', { cwds: [cwd], forceReload: true });
    assert.ok(
      skills.data.some((entry) =>
        entry.skills.some((skill) => skill.name === 'orchestrator-contract'),
      ),
    );
    const models = await server.call('model/list', {});
    assert.ok(models.data.some((model) => model.model === 'gpt-5.6-terra'));
    const quickSkill = skills.data
      .flatMap((entry) => entry.skills)
      .find((skill) => skill.name === 'orchestrator-contract');
    assert.equal(quickSkill.enabled, true);
    assert.equal(path.resolve(quickSkill.path), path.resolve(extraSkill, 'SKILL.md'));
    const quickModel = models.data.find((model) => model.model === 'gpt-5.6-terra');
    assert.ok(
      quickModel.supportedReasoningEfforts.some(
        (mode) => mode.reasoningEffort === quickModel.defaultReasoningEffort,
      ),
    );
    const started = await server.call('thread/start', { cwd });
    const threadId = started.thread.id;
    assert.equal(started.model, 'gpt-5.6-terra');
    assert.equal(started.reasoningEffort, 'low');
    assert.equal(started.cwd, cwd);
    assert.equal(started.approvalPolicy, 'on-request');
    const providerStarted = provider.nextRequest();
    const turn = (
      await server.call('turn/start', {
        threadId,
        input: [{ type: 'text', text: '$orchestrator-contract Reply with the fixture result.' }],
      })
    ).turn;
    const [firstRequest] = await providerStarted;
    assert.ok(
      JSON.stringify(firstRequest.input).includes('SKILL_ROOT_MARKER'),
      'An explicit skill mention loads its instructions into the provider input',
    );
    const steered = await server.call('turn/steer', {
      threadId,
      expectedTurnId: turn.id,
      input: [{ type: 'text', text: 'STEERING_MARKER' }],
    });
    assert.equal(steered.turnId, turn.id);
    const completed = await server.waitFor(
      (m) => m.method === 'turn/completed' && m.params.turn.id === turn.id,
    );
    assert.equal(completed.params.turn.status, 'completed');
    assert.equal(
      server.messages.filter((m) => m.method === 'turn/started' && m.params.turn.id === turn.id)
        .length,
      1,
    );
    assert.ok(
      provider.requests.some((request) =>
        JSON.stringify(request.input).includes('STEERING_MARKER'),
      ),
    );
    await assert.rejects(
      server.call('turn/steer', {
        threadId,
        expectedTurnId: turn.id,
        input: [{ type: 'text', text: 'Too late.' }],
      }),
      /no active turn/,
    );
    const nextProvider = provider.nextRequest();
    const interruptedTurn = (
      await server.call('turn/start', {
        threadId,
        input: [{ type: 'text', text: 'Wait for interruption.' }],
      })
    ).turn;
    await nextProvider;
    await server.call('turn/interrupt', { threadId, turnId: interruptedTurn.id });
    const interrupted = await server.waitFor(
      (m) => m.method === 'turn/completed' && m.params.turn.id === interruptedTurn.id,
    );
    assert.equal(interrupted.params.turn.status, 'interrupted');
    await server.close();

    await writeFile(
      path.join(home, 'config.toml'),
      configuration(provider.port, 'gpt-5.6-sol', 'medium', 'never'),
    );
    server = await connect();
    const fresh = await server.call('thread/start', { cwd, ephemeral: true });
    assert.equal(fresh.model, 'gpt-5.6-sol');
    assert.equal(fresh.reasoningEffort, 'medium');
    const resumed = await server.call('thread/resume', { threadId, cwd });
    assert.equal(resumed.thread.id, threadId);
    assert.equal(
      resumed.model,
      started.model,
      '0.144.0 keeps the prior model without an explicit resume selection',
    );
    assert.equal(resumed.reasoningEffort, started.reasoningEffort);
    assert.equal(resumed.approvalPolicy, 'never');
    await server.close();

    server = await connect();
    const projected = await server.call('thread/resume', {
      threadId,
      cwd,
      model: fresh.model,
      approvalPolicy: fresh.approvalPolicy,
      sandbox: 'read-only',
      config: { model_reasoning_effort: fresh.reasoningEffort },
    });
    assert.equal(projected.model, fresh.model);
    assert.equal(projected.reasoningEffort, fresh.reasoningEffort);
    assert.equal(projected.approvalPolicy, fresh.approvalPolicy);
    assert.equal(projected.thread.id, threadId);
    assert.deepEqual(projected.sandbox, fresh.sandbox);
    assert.ok((await readFile(path.join(home, 'config.toml'), 'utf8')).includes('gpt-5.6-sol'));
  },
);
