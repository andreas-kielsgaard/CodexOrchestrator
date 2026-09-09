import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const execFileAsync = promisify(execFile);
const toolRoot = path.dirname(fileURLToPath(import.meta.url));
const ownershipPrefix = 'REVIEW_APP_WEBVIEW_OWNER_V1:';
export async function assertOwnedDebugger({ executablePath, pid, debugPort }) {
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

export async function resolveTarget(debugUrl, targetUrl) {
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

export async function withProtocol(webSocketUrl, run) {
  const socket = new WebSocket(webSocketUrl);
  await once(socket, 'open');
  try {
    return await run(protocolClient(socket));
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
      60_000,
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
