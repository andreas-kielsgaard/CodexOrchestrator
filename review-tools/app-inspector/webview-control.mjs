#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const toolRoot = path.dirname(fileURLToPath(import.meta.url));
const ownershipPrefix = 'REVIEW_APP_WEBVIEW_OWNER_V1:';
const maxSelectorLength = 512;
const maxTextCharacters = 16_384;

async function main() {
  const { action, options } = parseArguments(process.argv.slice(2));
  if (action === 'help') return process.stdout.write(helpText());
  const debugUrl = loopbackUrl(required(options['debug-url'], '--debug-url'));
  const executablePath = requiredPath(options.exe, '--exe');
  const pid = positiveInteger(options.pid, '--pid');
  const targetUrl = required(options['target-url'], '--target-url');
  const selector = boundedSelector(required(options.selector, '--selector'));
  const text = action === 'type' ? await resolveText(options) : null;
  if (action === 'click' && (options.text || options['text-file'])) {
    throw new Error('--text and --text-file apply only to type.');
  }
  const ownership = await assertOwnedDebugger({
    executablePath,
    pid,
    debugPort: debuggerPort(debugUrl),
  });
  const target = await resolveTarget(debugUrl, targetUrl);
  const dispatch = await dispatchInput(target.webSocketDebuggerUrl, { action, selector, text });
  const receipt = {
    schemaVersion: 'review-app-webview-control/v2',
    observedAt: new Date().toISOString(),
    request: {
      debugUrl,
      executablePath: path.resolve(executablePath),
      pid,
      targetUrl,
      action,
      selector,
      text: text ? { characters: text.length, sha256: sha256(text) } : null,
    },
    target: { id: target.id, title: target.title, url: target.url },
    ownership,
    transport: {
      foregrounded: false,
      preDispatchOwnership: 'observed_before_dispatch',
      dispatchedInput: dispatch,
      ownershipBoundary:
        'ownership was verified before dispatch; it is not race-free identity proof',
      semanticOutcome:
        'not_observed; retain a separate visual, native-query, or provider observation',
    },
  };
  if (options.out) await writeReceipt(path.resolve(options.out), receipt);
  process.stdout.write(`${JSON.stringify(receipt, null, 2)}\n`);
}

async function assertOwnedDebugger({ executablePath, pid, debugPort }) {
  if (process.platform !== 'win32') {
    throw new Error('Owned WebView control currently supports Windows only.');
  }
  const script = path.join(toolRoot, 'adapters', 'windows-webview-owner.ps1');
  const { stdout } = await execFileAsync(
    'powershell.exe',
    [
      '-NoProfile',
      '-ExecutionPolicy',
      'Bypass',
      '-File',
      script,
      '-OwnerExecutablePath',
      executablePath,
      '-OwnerProcessId',
      String(pid),
      '-DebugPort',
      String(debugPort),
    ],
    { encoding: 'utf8', windowsHide: true, timeout: 15_000 },
  );
  return parseOwnershipOutput(stdout);
}

export function parseOwnershipOutput(stdout) {
  const frames = String(stdout ?? '')
    .split(/\r?\n/u)
    .filter((line) => line.startsWith(ownershipPrefix));
  if (frames.length !== 1) {
    throw new Error(
      `WebView owner adapter expected exactly one ${ownershipPrefix} frame; observed ${frames.length}.`,
    );
  }
  const encoded = frames[0].slice(ownershipPrefix.length);
  if (!/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/u.test(encoded)) {
    throw new Error('WebView owner adapter frame is not canonical base64.');
  }
  const bytes = Buffer.from(encoded, 'base64');
  if (bytes.toString('base64') !== encoded) {
    throw new Error('WebView owner adapter frame is not canonical base64.');
  }
  const value = JSON.parse(bytes.toString('utf8'));
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error('WebView owner adapter frame must contain a JSON object.');
  }
  return value;
}

async function resolveTarget(debugUrl, targetUrl) {
  const response = await fetch(new URL('/json/list', debugUrl));
  if (!response.ok)
    throw new Error(`WebView debugger target discovery failed: HTTP ${response.status}.`);
  const targets = await response.json();
  const matches = targets.filter((target) => target.type === 'page' && target.url === targetUrl);
  if (matches.length !== 1)
    throw new Error(
      `Expected exactly one page target with URL ${targetUrl}; observed ${matches.length}.`,
    );
  if (!matches[0].webSocketDebuggerUrl)
    throw new Error('Matched target exposes no WebSocket debugger URL.');
  return {
    ...matches[0],
    webSocketDebuggerUrl: validateWebSocketDebuggerUrl(matches[0].webSocketDebuggerUrl, debugUrl),
  };
}

export function validateWebSocketDebuggerUrl(value, debugUrl) {
  const endpoint = new URL(value);
  const debuggerEndpoint = new URL(debugUrl);
  if (endpoint.protocol !== 'ws:') {
    throw new Error('WebView target must expose a ws WebSocket debugger URL.');
  }
  if (!isLoopbackHost(endpoint.hostname)) {
    throw new Error('WebView target WebSocket host must be loopback.');
  }
  if (endpoint.port !== debuggerEndpoint.port) {
    throw new Error('WebView target WebSocket port must match the validated debugger port.');
  }
  if (endpoint.username || endpoint.password) {
    throw new Error('WebView target WebSocket URL must not include credentials.');
  }
  return endpoint.toString();
}

async function dispatchInput(webSocketUrl, { action, selector, text }) {
  const socket = new WebSocket(webSocketUrl);
  await once(socket, 'open');
  try {
    const protocol = protocolClient(socket);
    await protocol.request('DOM.enable');
    const node = await resolveSelector(protocol, selector);
    const dispatch =
      action === 'click'
        ? await dispatchClick(protocol, node)
        : await dispatchType(protocol, node, text);
    await new Promise((resolve) => setTimeout(resolve, 100));
    return dispatch;
  } finally {
    socket.close();
  }
}

function protocolClient(socket) {
  let nextId = 1;
  return {
    request(method, params = {}) {
      return request(socket, { id: nextId++, method, params });
    },
    dispatch(method, params = {}) {
      socket.send(JSON.stringify({ id: nextId++, method, params }));
    },
  };
}

function request(socket, payload) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new Error(`Chrome DevTools Protocol ${payload.method} request timed out.`)),
      10_000,
    );
    socket.addEventListener('message', (event) => {
      try {
        const message = JSON.parse(String(event.data));
        if (message.id !== payload.id) return;
        clearTimeout(timer);
        if (message.error) {
          reject(
            new Error(
              `Chrome DevTools Protocol ${payload.method} failed: ${message.error.message}`,
            ),
          );
        } else {
          resolve(message.result ?? {});
        }
      } catch (error) {
        clearTimeout(timer);
        reject(error);
      }
    });
    socket.addEventListener(
      'error',
      () => {
        clearTimeout(timer);
        reject(new Error(`Chrome DevTools Protocol ${payload.method} socket failed.`));
      },
      { once: true },
    );
    socket.send(JSON.stringify(payload));
  });
}

export async function resolveSelector(protocol, selector) {
  const document = await protocol.request('DOM.getDocument', { depth: 0, pierce: false });
  if (!document.root?.nodeId) throw new Error('WebView document root is unavailable.');
  const result = await protocol.request('DOM.querySelectorAll', {
    nodeId: document.root.nodeId,
    selector,
  });
  if (result.nodeIds?.length !== 1) {
    throw new Error(
      `Expected exactly one selector match for ${selector}; observed ${result.nodeIds?.length ?? 0}.`,
    );
  }
  const nodeId = result.nodeIds[0];
  if (!Number.isSafeInteger(nodeId) || nodeId <= 0) {
    throw new Error('Selector lookup did not return one DOM node.');
  }
  const described = await protocol.request('DOM.describeNode', {
    nodeId,
    depth: 0,
    pierce: false,
  });
  if (!described.node) throw new Error('Selector lookup node could not be described.');
  return { nodeId, node: described.node };
}

async function dispatchClick(protocol, target) {
  assertClickable(target.node);
  const point = await controlPoint(protocol, target.nodeId);
  dispatchPointerClick(protocol, point);
  return dispatchedReceipt(target.node, ['mouseMoved', 'mousePressed', 'mouseReleased']);
}

async function dispatchType(protocol, target, text) {
  assertTextEntry(target.node);
  const point = await controlPoint(protocol, target.nodeId);
  dispatchPointerClick(protocol, point);
  await new Promise((resolve) => setTimeout(resolve, 100));
  protocol.dispatch('Input.dispatchKeyEvent', {
    type: 'rawKeyDown',
    key: 'Control',
    code: 'ControlLeft',
    windowsVirtualKeyCode: 17,
    modifiers: 2,
  });
  protocol.dispatch('Input.dispatchKeyEvent', {
    type: 'keyDown',
    key: 'a',
    code: 'KeyA',
    windowsVirtualKeyCode: 65,
    modifiers: 2,
  });
  protocol.dispatch('Input.dispatchKeyEvent', {
    type: 'keyUp',
    key: 'a',
    code: 'KeyA',
    windowsVirtualKeyCode: 65,
    modifiers: 2,
  });
  protocol.dispatch('Input.dispatchKeyEvent', {
    type: 'keyUp',
    key: 'Control',
    code: 'ControlLeft',
    windowsVirtualKeyCode: 17,
  });
  protocol.dispatch('Input.insertText', { text });
  return dispatchedReceipt(target.node, [
    'mouseMoved',
    'mousePressed',
    'mouseReleased',
    'rawKeyDown',
    'keyDown',
    'keyUp',
    'Input.insertText',
  ]);
}

function dispatchPointerClick(protocol, point) {
  protocol.dispatch('Input.dispatchMouseEvent', {
    type: 'mouseMoved',
    x: point.x,
    y: point.y,
    button: 'none',
  });
  protocol.dispatch('Input.dispatchMouseEvent', {
    type: 'mousePressed',
    x: point.x,
    y: point.y,
    button: 'left',
    buttons: 1,
    clickCount: 1,
  });
  protocol.dispatch('Input.dispatchMouseEvent', {
    type: 'mouseReleased',
    x: point.x,
    y: point.y,
    button: 'left',
    buttons: 0,
    clickCount: 1,
  });
}

async function controlPoint(protocol, nodeId) {
  const resolved = await protocol.request('DOM.resolveNode', { nodeId });
  const objectId = resolved.object?.objectId;
  if (!objectId) throw new Error('Selector target cannot be resolved for geometry.');
  try {
    const measured = await protocol.request('Runtime.callFunctionOn', {
      objectId,
      functionDeclaration:
        'function() { const rect = this.getBoundingClientRect(); return { left: rect.left, right: rect.right, top: rect.top, bottom: rect.bottom }; }',
      returnByValue: true,
      silent: true,
    });
    const rect = measured.result?.value;
    if (!rect || ![rect.left, rect.right, rect.top, rect.bottom].every(Number.isFinite)) {
      throw new Error('Selector target has no usable visible geometry.');
    }
    const x = (rect.left + rect.right) / 2;
    const y = (rect.top + rect.bottom) / 2;
    if (
      !Number.isFinite(x) ||
      !Number.isFinite(y) ||
      rect.right <= rect.left ||
      rect.bottom <= rect.top
    ) {
      throw new Error('Selector target has empty or invalid geometry.');
    }
    return { x, y };
  } finally {
    await protocol.request('Runtime.releaseObject', { objectId }).catch(() => {});
  }
}

function assertClickable(node) {
  const name = String(node.localName ?? node.nodeName ?? '').toLowerCase();
  const attributes = attributesFor(node);
  if (attributes.has('disabled')) throw new Error('Selected control is disabled.');
  const inputType = (attributes.get('type') ?? 'text').toLowerCase();
  const supported =
    name === 'button' ||
    name === 'a' ||
    (name === 'input' && ['button', 'checkbox', 'radio', 'reset', 'submit'].includes(inputType));
  if (!supported)
    throw new Error(`Selected element ${name || 'unknown'} is not a supported click control.`);
}

function assertTextEntry(node) {
  const name = String(node.localName ?? node.nodeName ?? '').toLowerCase();
  const attributes = attributesFor(node);
  if (attributes.has('disabled') || attributes.has('readonly')) {
    throw new Error('Selected text control is disabled or read-only.');
  }
  const inputType = (attributes.get('type') ?? 'text').toLowerCase();
  const supportedInputTypes = ['email', 'search', 'tel', 'text', 'url'];
  if (name !== 'textarea' && !(name === 'input' && supportedInputTypes.includes(inputType))) {
    throw new Error(`Selected element ${name || 'unknown'} is not a supported text control.`);
  }
}

function attributesFor(node) {
  const values = Array.isArray(node.attributes) ? node.attributes : [];
  const attributes = new Map();
  for (let index = 0; index + 1 < values.length; index += 2) {
    attributes.set(String(values[index]).toLowerCase(), String(values[index + 1]));
  }
  return attributes;
}

function dispatchedReceipt(node, events) {
  return {
    status: 'input_domain_commands_sent',
    nodeName: String(node.nodeName ?? '').toLowerCase(),
    events,
    boundary:
      'CDP command send proves neither trusted event delivery nor product semantics; retain separate observation.',
  };
}

function once(socket, eventName) {
  return new Promise((resolve, reject) => {
    socket.addEventListener(eventName, resolve, { once: true });
    socket.addEventListener(
      'error',
      () => reject(new Error('Chrome DevTools Protocol socket could not open.')),
      { once: true },
    );
  });
}

function parseArguments(args) {
  const [action = 'help', ...rest] = args;
  if (action === '--help' || action === '-h') return { action: 'help', options: {} };
  if (!['type', 'click'].includes(action))
    throw new Error(`Unknown action: ${action}. Run with --help for usage.`);
  const options = {};
  for (let index = 0; index < rest.length; index += 1) {
    const token = rest[index];
    if (!token.startsWith('--')) throw new Error(`Unexpected argument: ${token}`);
    const value = rest[index + 1];
    if (!value || value.startsWith('--')) throw new Error(`Missing value for ${token}`);
    options[token.slice(2)] = value;
    index += 1;
  }
  return { action, options };
}

async function resolveText(options) {
  if (Boolean(options.text) === Boolean(options['text-file']))
    throw new Error('type requires exactly one of --text or --text-file.');
  const text = options.text ?? (await readFile(path.resolve(options['text-file']), 'utf8'));
  if (text.length > maxTextCharacters) {
    throw new Error(`type text must not exceed ${maxTextCharacters} characters.`);
  }
  return text;
}

export function boundedSelector(value) {
  if (value.length > maxSelectorLength) {
    throw new Error(`--selector must not exceed ${maxSelectorLength} characters.`);
  }
  if (!value.trim()) throw new Error('--selector must not be blank.');
  return value;
}

export function loopbackUrl(value) {
  const parsed = new URL(value);
  if (parsed.protocol !== 'http:' || !isLoopbackHost(parsed.hostname)) {
    throw new Error('--debug-url must use an http loopback host.');
  }
  if (!parsed.port) throw new Error('--debug-url must include an explicit debugger port.');
  if (parsed.username || parsed.password)
    throw new Error('--debug-url must not include credentials.');
  return parsed.toString().replace(/\/$/u, '');
}
function isLoopbackHost(hostname) {
  return ['127.0.0.1', 'localhost', '[::1]'].includes(hostname);
}
export function debuggerPort(debugUrl) {
  const port = Number(new URL(debugUrl).port);
  if (!Number.isSafeInteger(port) || port < 1 || port > 65535) {
    throw new Error('--debug-url must include a valid debugger port.');
  }
  return port;
}

function required(value, name) {
  if (!value) throw new Error(`${name} is required.`);
  return value;
}
function requiredPath(value, name) {
  if (!value) throw new Error(`${name} is required.`);
  return value;
}
function positiveInteger(value, name) {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isSafeInteger(parsed) || parsed <= 0 || String(parsed) !== String(value)) {
    throw new Error(`${name} must be a positive integer.`);
  }
  return parsed;
}
function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}
async function writeReceipt(filePath, receipt) {
  await mkdir(path.dirname(filePath), { recursive: true });
  const temporary = `${filePath}.tmp`;
  await writeFile(temporary, `${JSON.stringify(receipt, null, 2)}\n`, 'utf8');
  await rename(temporary, filePath);
}
function helpText() {
  return `Codex Orchestrator owned loopback WebView control companion\n\nUse only against an isolated development instance launched with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<port>:\n  node review-tools/app-inspector/webview-control.mjs type --exe <absolute-owner-exe> --pid <owner-pid> --debug-url http://127.0.0.1:<port> --target-url http://127.0.0.1:1420/ --selector 'textarea' --text-file <utf8-file> --out <receipt.json>\n  node review-tools/app-inspector/webview-control.mjs click --exe <absolute-owner-exe> --pid <owner-pid> --debug-url http://127.0.0.1:<port> --target-url http://127.0.0.1:1420/ --selector 'button[type="submit"]' --out <receipt.json>\n\nThe tool permits only HTTP loopback debugger endpoints with an explicit port. Before dispatch it verifies that the selected owner EXE and PID are live, exactly one loopback listener endpoint belongs to a descendant process and declares the requested port. It requires exactly one page target URL and one bounded selector, derives a point from that selected control with a fixed internal geometry query, and dispatches only CDP Input events. It exposes no coordinate or arbitrary-script interface. Text is redacted by length/hash. Pre-dispatch ownership, input dispatch, and product semantics are separate receipt facts; the receipt proves no semantic outcome.\n`;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    process.stderr.write(
      `webview-control: ${error instanceof Error ? error.message : String(error)}\n`,
    );
    process.exitCode = 1;
  });
}
