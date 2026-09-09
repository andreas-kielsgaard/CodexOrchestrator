#!/usr/bin/env node
import {
  assertOwnedDebugger,
  resolveTarget,
  withProtocol,
  loopbackUrl,
  debuggerPort,
} from './owned-webview.mjs';
export {
  parseOwnershipOutput,
  validateWebSocketDebuggerUrl,
  loopbackUrl,
  debuggerPort,
} from './owned-webview.mjs';

import { createHash } from 'node:crypto';
import { mkdir, readFile, rename, writeFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { captureRenderedState } from './src/rendered-state.mjs';

const maxSelectorLength = 512;
const maxTextCharacters = 16_384;

async function main() {
  const { action, options } = parseArguments(process.argv.slice(2));
  if (action === 'help') return process.stdout.write(helpText());
  const debugUrl = loopbackUrl(required(options['debug-url'], '--debug-url'));
  const executablePath = requiredPath(options.exe, '--exe');
  const pid = positiveInteger(options.pid, '--pid');
  const targetUrl = required(options['target-url'], '--target-url');
  const selector =
    action === 'snapshot' ? null : boundedSelector(required(options.selector, '--selector'));
  const text = action === 'type' ? await resolveText(options) : null;
  const value = action === 'select' ? required(options.value, '--value') : null;
  if (action !== 'select' && options.value) throw new Error('--value applies only to select.');
  if (action !== 'type' && (options.text || options['text-file'])) {
    throw new Error('--text and --text-file apply only to type.');
  }
  const ownership = await assertOwnedDebugger({
    executablePath,
    pid,
    debugPort: debuggerPort(debugUrl),
  });
  const target = await resolveTarget(debugUrl, targetUrl);
  if (action === 'snapshot') {
    if (options.selector)
      throw new Error('snapshot captures the rendered page, without a selector.');
    const snapshot = await withProtocol(target.webSocketDebuggerUrl, (protocol) =>
      captureRenderedState(protocol, options.screenshot ? path.resolve(options.screenshot) : null),
    );
    const receipt = {
      schemaVersion: 'review-app-rendered-state/v1',
      observedAt: new Date().toISOString(),
      request: { executablePath, pid, debugUrl, targetUrl },
      target: { id: target.id, title: target.title, url: target.url },
      ownership,
      ...snapshot,
      boundaries: {
        readOnly: true,
        foregrounded: false,
        inputDispatched: false,
        observation:
          'DOM, accessibility and screenshot are sequential point-in-time reads; no durable or provider outcome is inferred.',
      },
    };
    if (options.out) await writeReceipt(path.resolve(options.out), receipt);
    process.stdout.write(`${JSON.stringify(receipt, null, 2)}\n`);
    return;
  }
  if (options.screenshot) throw new Error('--screenshot applies only to snapshot.');
  const dispatch = await dispatchInput(target.webSocketDebuggerUrl, {
    action,
    selector,
    text,
    value,
  });
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
      value,
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

async function dispatchInput(webSocketUrl, { action, selector, text, value }) {
  return withProtocol(webSocketUrl, async (protocol) => {
    await protocol.request('DOM.enable');
    const node = await resolveSelector(protocol, selector);
    const dispatch =
      action === 'select'
        ? await dispatchSelect(protocol, node, value)
        : action === 'click'
          ? await dispatchClick(protocol, node)
          : await dispatchType(protocol, node, text);
    await new Promise((resolve) => setTimeout(resolve, 100));
    return dispatch;
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
  const point = await scrollAndMeasureActionablePoint(protocol, target.nodeId, false);
  dispatchPointerClick(protocol, point);
  return dispatchedReceipt(target.node, ['mouseMoved', 'mousePressed', 'mouseReleased']);
}

export async function dispatchSelect(protocol, target, value) {
  if (
    String(target.node.nodeName).toLowerCase() !== 'select' ||
    attributesFor(target.node).has('multiple')
  )
    throw new Error('select requires a single-selection select control.');
  await scrollAndMeasureActionablePoint(protocol, target.nodeId);
  const resolved = await protocol.request('DOM.resolveNode', { nodeId: target.nodeId });
  const objectId = resolved.object?.objectId;
  if (!objectId) throw new Error('Select control could not be resolved.');
  let index;
  try {
    const result = await protocol.request('Runtime.callFunctionOn', {
      objectId,
      functionDeclaration: `function() { return Array.from(this.options).filter(o => !o.disabled && !o.closest('optgroup[disabled]') && !o.hidden).map(o => o.value); }`,
      returnByValue: true,
      silent: true,
    });
    const values = result.result?.value;
    if (!Array.isArray(values) || values.filter((v) => v === value).length !== 1)
      throw new Error('Expected one enabled option with the requested value.');
    index = values.indexOf(value);
  } finally {
    await protocol.request('Runtime.releaseObject', { objectId }).catch(() => {});
  }
  await protocol.request('DOM.focus', { nodeId: target.nodeId });
  for (const key of ['Home', ...Array(index).fill('ArrowDown')]) {
    const code = key === 'Home' ? 36 : 40;
    await protocol.request('Input.dispatchKeyEvent', {
      type: 'rawKeyDown',
      key,
      code: key,
      windowsVirtualKeyCode: code,
    });
    await protocol.request('Input.dispatchKeyEvent', {
      type: 'keyUp',
      key,
      code: key,
      windowsVirtualKeyCode: code,
    });
  }
  return dispatchedReceipt(target.node, ['focus selected control', 'Home', `${index} ArrowDown`]);
}

async function dispatchType(protocol, target, text) {
  assertTextEntry(target.node);
  const point = await scrollAndMeasureActionablePoint(protocol, target.nodeId, true);
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

export async function scrollAndMeasureActionablePoint(protocol, nodeId, textEntry = false) {
  await protocol.request('DOM.scrollIntoViewIfNeeded', { nodeId, centerIfNeeded: true });
  const resolved = await protocol.request('DOM.resolveNode', { nodeId });
  const objectId = resolved.object?.objectId;
  if (!objectId) throw new Error('Selector target cannot be resolved for geometry.');
  try {
    const measured = await protocol.request('Runtime.callFunctionOn', {
      objectId,
      functionDeclaration: `function() {
        const rect = this.getBoundingClientRect(); const style = getComputedStyle(this);
        const candidates = [this, ...Array.from(this.querySelectorAll('line,path,rect')).slice(0, 100)];
        let x = -1, y = -1, covered = true;
        for (const candidate of candidates) {
          const box = candidate.getBoundingClientRect();
          const px = (Math.max(0, box.left) + Math.min(innerWidth, box.right)) / 2;
          const py = (Math.max(0, box.top) + Math.min(innerHeight, box.bottom)) / 2;
          const hit = document.elementFromPoint(px, py);
          if (hit === this || this.contains(hit)) { x = px; y = py; covered = false; break; }
        }
        return { connected: this.isConnected, disabled: Boolean(this.disabled) || this.getAttribute('aria-disabled') === 'true',
          readOnly: Boolean(this.readOnly), display: style.display, visibility: style.visibility, opacity: style.opacity,
          clientRects: this.getClientRects().length, left: rect.left, right: rect.right, top: rect.top, bottom: rect.bottom,
          viewportWidth: innerWidth, viewportHeight: innerHeight, x, y, covered };
      }`,
      returnByValue: true,
      silent: true,
    });
    const rect = measured.result?.value;
    if (
      !rect ||
      ![
        rect.left,
        rect.right,
        rect.top,
        rect.bottom,
        rect.viewportWidth,
        rect.viewportHeight,
        rect.x,
        rect.y,
      ].every(Number.isFinite)
    ) {
      throw new Error('Selector target has no usable visible geometry.');
    }
    if (
      !rect.connected ||
      rect.disabled ||
      (textEntry && rect.readOnly) ||
      rect.display === 'none' ||
      rect.visibility === 'hidden' ||
      rect.visibility === 'collapse' ||
      Number(rect.opacity) <= 0 ||
      rect.clientRects < 1 ||
      rect.right <= rect.left ||
      rect.bottom <= rect.top ||
      rect.viewportWidth <= 0 ||
      rect.viewportHeight <= 0 ||
      rect.x < 0 ||
      rect.x >= rect.viewportWidth ||
      rect.y < 0 ||
      rect.y >= rect.viewportHeight ||
      rect.covered
    ) {
      throw new Error('Selector target is not an actionable visible control.');
    }
    return { x: rect.x, y: rect.y };
  } finally {
    await protocol.request('Runtime.releaseObject', { objectId }).catch(() => {});
  }
}

export function assertClickable(node) {
  const name = String(node.localName ?? node.nodeName ?? '').toLowerCase();
  const attributes = attributesFor(node);
  if (attributes.has('disabled')) throw new Error('Selected control is disabled.');
  const inputType = (attributes.get('type') ?? 'text').toLowerCase();
  const supported =
    name === 'button' ||
    name === 'a' ||
    ['button', 'tab'].includes(attributes.get('role')) ||
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

function parseArguments(args) {
  const [action = 'help', ...rest] = args;
  if (action === '--help' || action === '-h') return { action: 'help', options: {} };
  if (!['type', 'click', 'select', 'snapshot'].includes(action))
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
  return `Codex Orchestrator owned loopback WebView review companion

Use an isolated development instance launched with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<port>.
All commands require --exe <absolute-owner-exe> --pid <owner-pid> --debug-url http://127.0.0.1:<port> --target-url http://127.0.0.1:1420/.

  snapshot [owner options] --out <state.json> [--screenshot <screen.png>]
  click [owner options] --selector 'button[type="submit"]' --out <receipt.json>
  type [owner options] --selector 'textarea' --text-file <utf8-file> --out <receipt.json>
  select [owner options] --selector 'select' --value <option-value> --out <receipt.json>

Snapshot reads rendered text, controls/selectors/geometry, focus, scroll regions and accessibility state without scrolling, input or foreground changes. Optional PNG capture reads the WebView compositor, excluding native window chrome. Password values are redacted; other visible page content is included. These sequential reads do not prove durable or provider outcomes.

The tool verifies the selected EXE and PID, the loopback listener's process ownership, and exactly one matching page URL before each operation. Click/type/select resolve one bounded selector and send CDP Input events to that control. Select focuses a single-selection control and uses Home/ArrowDown keys through its enabled options. No coordinate or arbitrary-script interface is exposed. Input receipts redact text by length/hash and distinguish ownership checks, sent commands and unobserved product semantics.
`;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    process.stderr.write(
      `webview-control: ${error instanceof Error ? error.message : String(error)}\n`,
    );
    process.exitCode = 1;
  });
}
