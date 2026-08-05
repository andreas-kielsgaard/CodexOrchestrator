#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

async function main() {
  const { action, options } = parseArguments(process.argv.slice(2));
  if (action === 'help') return process.stdout.write(helpText());
  const debugUrl = loopbackUrl(required(options['debug-url'], '--debug-url'));
  const targetUrl = required(options['target-url'], '--target-url');
  const selector = required(options.selector, '--selector');
  const text = action === 'type' ? await resolveText(options) : null;
  if (action === 'click' && (options.text || options['text-file'])) {
    throw new Error('--text and --text-file apply only to type.');
  }
  const target = await resolveTarget(debugUrl, targetUrl);
  const expression = action === 'type' ? typeExpression(selector, text) : clickExpression(selector);
  const result = await evaluate(target.webSocketDebuggerUrl, expression);
  if (result?.exceptionDetails) {
    const details = result.exceptionDetails;
    throw new Error(
      `WebView evaluation failed: ${details.exception?.description ?? details.exception?.value ?? details.text ?? 'unknown exception'}`,
    );
  }
  const receipt = {
    schemaVersion: 'review-app-webview-control/v1',
    observedAt: new Date().toISOString(),
    request: {
      debugUrl,
      targetUrl,
      action,
      selector,
      text: text ? { characters: text.length, sha256: sha256(text) } : null,
    },
    target: { id: target.id, title: target.title, url: target.url },
    transport: {
      foregrounded: false,
      delivery: 'Chrome DevTools Protocol Runtime.evaluate completed',
      semanticOutcome:
        'not_observed; retain a separate visual, native-query, or provider observation',
    },
  };
  if (options.out) await writeReceipt(path.resolve(options.out), receipt);
  process.stdout.write(`${JSON.stringify(receipt, null, 2)}\n`);
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
  return matches[0];
}

async function evaluate(webSocketUrl, expression) {
  const socket = new WebSocket(webSocketUrl);
  await once(socket, 'open');
  try {
    const response = await request(socket, {
      id: 1,
      method: 'Runtime.evaluate',
      params: { expression, awaitPromise: true, returnByValue: true },
    });
    if (response.error)
      throw new Error(`Chrome DevTools Protocol error: ${response.error.message}`);
    return response.result;
  } finally {
    socket.close();
  }
}

function request(socket, payload) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new Error('Chrome DevTools Protocol request timed out.')),
      10_000,
    );
    socket.addEventListener('message', (event) => {
      try {
        const message = JSON.parse(String(event.data));
        if (message.id !== payload.id) return;
        clearTimeout(timer);
        resolve(message);
      } catch (error) {
        clearTimeout(timer);
        reject(error);
      }
    });
    socket.addEventListener(
      'error',
      () => {
        clearTimeout(timer);
        reject(new Error('Chrome DevTools Protocol socket failed.'));
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

export function typeExpression(selector, text) {
  return `(() => { const element = document.querySelector(${JSON.stringify(selector)}); if (!element) throw new Error('selector_not_found'); const descriptor = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(element), 'value'); if (!descriptor?.set) throw new Error('value_setter_unavailable'); descriptor.set.call(element, ${JSON.stringify(text)}); element.dispatchEvent(new Event('input', { bubbles: true })); element.dispatchEvent(new Event('change', { bubbles: true })); return { matched: true, tagName: element.tagName }; })()`;
}

export function clickExpression(selector) {
  return `(() => { const element = document.querySelector(${JSON.stringify(selector)}); if (!element) throw new Error('selector_not_found'); element.click(); return { matched: true, tagName: element.tagName }; })()`;
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
  return options.text ?? readFile(path.resolve(options['text-file']), 'utf8');
}

export function loopbackUrl(value) {
  const parsed = new URL(value);
  if (
    !['http:', 'https:'].includes(parsed.protocol) ||
    !['127.0.0.1', 'localhost', '[::1]'].includes(parsed.hostname)
  ) {
    throw new Error('--debug-url must use an http(s) loopback host.');
  }
  return parsed.toString().replace(/\/$/u, '');
}

function required(value, name) {
  if (!value) throw new Error(`${name} is required.`);
  return value;
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
  return `Codex Orchestrator loopback WebView control companion\n\nUse only against an isolated development instance launched with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<port>:\n  node review-tools/app-inspector/webview-control.mjs type --debug-url http://127.0.0.1:<port> --target-url http://127.0.0.1:1420/ --selector 'textarea' --text-file <utf8-file> --out <receipt.json>\n  node review-tools/app-inspector/webview-control.mjs click --debug-url http://127.0.0.1:<port> --target-url http://127.0.0.1:1420/ --selector 'button[type="submit"]' --out <receipt.json>\n\nThe tool permits loopback debugger endpoints only, requires exactly one page target by URL, never foregrounds the window, and redacts entered text from receipts. It proves dispatch only, not UI or application semantics.\n`;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    process.stderr.write(
      `webview-control: ${error instanceof Error ? error.message : String(error)}\n`,
    );
    process.exitCode = 1;
  });
}
