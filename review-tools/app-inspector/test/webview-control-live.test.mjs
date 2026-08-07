import assert from 'node:assert/strict';
import { execFile, spawn } from 'node:child_process';
import { access, mkdtemp, readFile, readdir, rm } from 'node:fs/promises';
import http from 'node:http';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const here = path.dirname(fileURLToPath(import.meta.url));
const control = path.resolve(here, '..', 'webview-control.mjs');

test('dispatches trusted input only to the selected isolated Chromium target', async (t) => {
  const browserExecutable = await findChromiumDebugHost();
  if (!browserExecutable) return t.skip('Microsoft Edge is unavailable.');

  const root = await mkdtemp(path.join(os.tmpdir(), 'webview-control-live-'));
  const profile = path.join(root, 'profile');
  const receipts = path.join(root, 'receipts');
  const records = [];
  const server = http.createServer((request, response) => {
    const url = new URL(request.url, `http://${request.headers.host}`);
    if (url.pathname === '/record') {
      records.push({ target: url.searchParams.get('target'), kind: url.searchParams.get('kind') });
      response.writeHead(204).end();
      return;
    }
    if (url.pathname === '/target' || url.pathname === '/foreign') {
      const target = url.pathname.slice(1);
      response
        .writeHead(200, { 'content-type': 'text/html; charset=utf-8' })
        .end(fixtureHtml(target));
      return;
    }
    response.writeHead(404).end();
  });
  const serverPort = await listen(server);
  const debugPort = await reserveThenReleasePort();
  const targetUrl = `http://127.0.0.1:${serverPort}/target`;
  const browser = spawn(
    browserExecutable,
    [
      `--remote-debugging-port=${debugPort}`,
      '--remote-allow-origins=*',
      `--user-data-dir=${profile}`,
      '--disable-popup-blocking',
      `--app=${targetUrl}`,
    ],
    { windowsHide: true, stdio: 'ignore' },
  );

  try {
    await waitForTarget(`http://127.0.0.1:${debugPort}`, targetUrl);
    await createForeignTarget(
      `http://127.0.0.1:${debugPort}`,
      `http://127.0.0.1:${serverPort}/foreign`,
    );
    await activateTarget(`http://127.0.0.1:${debugPort}`, targetUrl);
    assert.equal(
      await evaluateTarget(`http://127.0.0.1:${debugPort}`, targetUrl, 'window.scrollY'),
      0,
    );
    const text = 'test-owned-text-is-redacted';
    const typedReceipt = path.join(receipts, 'type.json');
    const clickedReceipt = path.join(receipts, 'click.json');
    await runControl([
      'type',
      '--exe',
      process.execPath,
      '--pid',
      String(process.pid),
      '--debug-url',
      `http://127.0.0.1:${debugPort}`,
      '--target-url',
      targetUrl,
      '--selector',
      '#target-input',
      '--text',
      text,
      '--out',
      typedReceipt,
    ]);
    await waitForRecords(records, 1);
    await runControl([
      'click',
      '--exe',
      process.execPath,
      '--pid',
      String(process.pid),
      '--debug-url',
      `http://127.0.0.1:${debugPort}`,
      '--target-url',
      targetUrl,
      '--selector',
      '#target-button',
      '--out',
      clickedReceipt,
    ]);
    await waitForRecords(records, 2);

    const targetState = await evaluateTargetState(`http://127.0.0.1:${debugPort}`, targetUrl);
    assert.deepEqual(targetState, {
      trustedInput: true,
      trustedClick: true,
      status: 'trusted click',
    });
    assert.ok(
      await evaluateTarget(`http://127.0.0.1:${debugPort}`, targetUrl, 'window.scrollY'),
      'the trusted click scrolls the below-fold target into view first',
    );
    assert.deepEqual(
      await evaluateTargetState(
        `http://127.0.0.1:${debugPort}`,
        `http://127.0.0.1:${serverPort}/foreign`,
      ),
      { trustedInput: false, trustedClick: false, status: 'idle' },
    );
    assert.deepEqual(records, [
      { target: 'target', kind: 'input' },
      { target: 'target', kind: 'click' },
    ]);
    const receiptText = await readFile(typedReceipt, 'utf8');
    assert.doesNotMatch(receiptText, new RegExp(text, 'u'));
    const receipt = JSON.parse(receiptText);
    assert.equal(receipt.request.text.characters, text.length);
    assert.equal(receipt.transport.preDispatchOwnership, 'observed_before_dispatch');
    assert.equal(receipt.transport.dispatchedInput.status, 'input_domain_commands_sent');
    assert.equal(
      receipt.transport.semanticOutcome,
      'not_observed; retain a separate visual, native-query, or provider observation',
    );
    assert.match(await readFile(clickedReceipt, 'utf8'), /mousePressed/u);
  } finally {
    await stopOwnedTestBrowser(browser);
    await close(server);
    await rm(root, { recursive: true, force: true });
  }
});

async function findChromiumDebugHost() {
  const root = process.env['ProgramFiles(x86)'];
  if (!root) return null;
  const application = path.join(root, 'Microsoft', 'Edge', 'Application');
  try {
    const versions = [...(await readdir(application)).sort(), ''];
    for (const version of versions) {
      const candidate = path.join(application, version, 'msedge.exe');
      try {
        await access(candidate);
        return candidate;
      } catch {
        // Continue through the runtime versions exposed on this test machine.
      }
    }
  } catch {
    return null;
  }
  return null;
}

function fixtureHtml(target) {
  return `<!doctype html><html><body><label>${target}<input id="${target}-input"></label><div style="height: 2400px"></div><button id="${target}-button" type="button">below fold</button><output id="status">idle</output><script>window.trustedInput=false;window.trustedClick=false;const report=(kind)=>fetch('/record?target=${target}&kind='+kind);document.querySelector('#${target}-input').addEventListener('input',(event)=>{if(event.isTrusted){window.trustedInput=true;document.querySelector('#status').textContent='trusted input';report('input')}});document.querySelector('#${target}-button').addEventListener('click',(event)=>{if(event.isTrusted){window.trustedClick=true;document.querySelector('#status').textContent='trusted click';report('click')}});</script></body></html>`;
}

async function createForeignTarget(debugUrl, foreignUrl) {
  const response = await fetch(new URL(`/json/new?${encodeURIComponent(foreignUrl)}`, debugUrl), {
    method: 'PUT',
  });
  if (!response.ok)
    throw new Error(`fixture foreign target creation failed: HTTP ${response.status}`);
}

async function activateTarget(debugUrl, targetUrl) {
  const targets = await (await fetch(new URL('/json/list', debugUrl))).json();
  const target = targets.find(
    (candidate) => candidate.type === 'page' && candidate.url === targetUrl,
  );
  if (!target?.id) throw new Error('fixture selected target was lost before activation');
  const response = await fetch(new URL(`/json/activate/${target.id}`, debugUrl));
  if (!response.ok) throw new Error(`fixture target activation failed: HTTP ${response.status}`);
}

async function runControl(args) {
  await execFileAsync(process.execPath, [control, ...args], { windowsHide: true });
}

async function waitForTarget(debugUrl, targetUrl) {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    try {
      const targets = await (await fetch(new URL('/json/list', debugUrl))).json();
      if (targets.some((target) => target.type === 'page' && target.url === targetUrl)) return;
    } catch {
      // The test-owned Chromium target has not opened its debugger endpoint yet.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error('Timed out waiting for the test-owned Chromium debug target.');
}

async function waitForRecords(records, count) {
  const deadline = Date.now() + 5_000;
  while (Date.now() < deadline) {
    if (records.length >= count) return;
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
  throw new Error(
    `Timed out waiting for ${count} trusted fixture events; observed ${records.length}.`,
  );
}

async function evaluateTargetState(debugUrl, targetUrl) {
  return evaluateTarget(
    debugUrl,
    targetUrl,
    '({ trustedInput: window.trustedInput, trustedClick: window.trustedClick, status: document.querySelector("#status").textContent })',
  );
}

async function evaluateTarget(debugUrl, targetUrl, expression) {
  const targets = await (await fetch(new URL('/json/list', debugUrl))).json();
  const target = targets.find(
    (candidate) => candidate.type === 'page' && candidate.url === targetUrl,
  );
  assert.ok(target?.webSocketDebuggerUrl, 'the selected test target exposes a debugger WebSocket');
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await once(socket, 'open');
  try {
    const response = await request(socket, {
      id: 1,
      method: 'Runtime.evaluate',
      params: {
        expression,
        returnByValue: true,
      },
    });
    return response.result.result.value;
  } finally {
    socket.close();
  }
}

function request(socket, payload) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('fixture CDP request timed out')), 10_000);
    socket.addEventListener('message', (event) => {
      const message = JSON.parse(String(event.data));
      if (message.id !== payload.id) return;
      clearTimeout(timer);
      if (message.error) reject(new Error(message.error.message));
      else resolve(message);
    });
    socket.addEventListener('error', () => reject(new Error('fixture CDP socket failed')), {
      once: true,
    });
    socket.send(JSON.stringify(payload));
  });
}

function once(socket, eventName) {
  return new Promise((resolve, reject) => {
    socket.addEventListener(eventName, resolve, { once: true });
    socket.addEventListener('error', () => reject(new Error('fixture CDP socket could not open')), {
      once: true,
    });
  });
}

async function reserveThenReleasePort() {
  const server = net.createServer();
  const port = await listen(server);
  await close(server);
  return port;
}

function listen(server) {
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen({ host: '127.0.0.1', port: 0 }, () => resolve(server.address().port));
  });
}

function close(server) {
  return new Promise((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
}

async function stopOwnedTestBrowser(browser) {
  if (browser.exitCode !== null) return;
  browser.kill();
  await new Promise((resolve) => setTimeout(resolve, 250));
  if (browser.exitCode === null) {
    await execFileAsync('taskkill.exe', ['/PID', String(browser.pid), '/T', '/F'], {
      windowsHide: true,
    }).catch(() => {});
  }
}
