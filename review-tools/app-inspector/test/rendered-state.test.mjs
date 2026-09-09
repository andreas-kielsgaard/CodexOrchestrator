import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { JSDOM } from 'jsdom';
import { captureRenderedState, readRenderedPage } from '../src/rendered-state.mjs';

test('reports labeled controls and state while excluding hidden controls and password values', () => {
  const dom = new JSDOM(
    '<body><label for="name">Task name</label><input id="name" value="Example"><input id="secret" type="password" value="never-retain"><button id="disabled" disabled>Unavailable</button><section hidden><button>Hidden</button></section><button role="tab" aria-selected="true">OTP configuration</button></body>',
    { runScripts: 'outside-only' },
  );
  const { window } = dom;
  window.CSS = { escape: (value) => value };
  const query = window.document.querySelector.bind(window.document);
  window.document.querySelector = (selector) =>
    selector === 'dialog:modal' ? null : query(selector);
  Object.defineProperty(window.HTMLElement.prototype, 'innerText', {
    get() {
      return this.textContent;
    },
  });
  window.Element.prototype.getClientRects = () => [{ x: 0, y: 0, width: 100, height: 30 }];
  window.Element.prototype.getBoundingClientRect = () => ({ x: 0, y: 0, width: 100, height: 30 });
  window.document.elementFromPoint = () => null;
  const state = window.eval(`(${readRenderedPage.toString()})()`);
  assert.equal(state.controls.find((c) => c.selector === '#name').name, 'Task name');
  assert.equal(state.controls.find((c) => c.selector === '#name').value, 'Example');
  assert.equal(state.controls.find((c) => c.selector === '#secret').value, '[redacted]');
  assert.equal(state.controls.find((c) => c.selector === '#disabled').disabled, true);
  assert.equal(state.controls.find((c) => c.name === 'OTP configuration').selected, 'true');
  assert.equal(
    state.controls.some((c) => c.name === 'Hidden'),
    false,
  );
  assert.doesNotMatch(JSON.stringify(state), /never-retain/);
  window.document.body.insertAdjacentHTML(
    'beforeend',
    '<form role="dialog" aria-modal="true"><button>Cancel</button></form>',
  );
  const modalState = window.eval(`(${readRenderedPage.toString()})()`);
  assert.equal(modalState.controls.find((c) => c.selector === '#name').blockedByModal, true);
  assert.equal(modalState.controls.find((c) => c.name === 'Cancel').blockedByModal, false);
  dom.window.close();
});

test('captures compositor image and semantic state with read-only protocol commands', async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), 'rendered-state-'));
  const png = Buffer.from(
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+X2ioAAAAASUVORK5CYII=',
    'base64',
  );
  const calls = [];
  try {
    const result = await captureRenderedState(
      {
        request: async (method) => {
          calls.push(method);
          if (method === 'Runtime.evaluate')
            return { result: { value: { text: 'OTP configuration' } } };
          if (method === 'Accessibility.getFullAXTree')
            return {
              nodes: [
                {
                  nodeId: '1',
                  ignored: false,
                  role: { value: 'button' },
                  name: { value: 'Refresh' },
                },
              ],
            };
          if (method === 'Page.captureScreenshot') return { data: png.toString('base64') };
          throw new Error(method);
        },
      },
      path.join(root, 'screen.png'),
    );
    assert.deepEqual(calls, [
      'Runtime.evaluate',
      'Accessibility.getFullAXTree',
      'Page.captureScreenshot',
    ]);
    assert.equal(result.rendered.text, 'OTP configuration');
    assert.equal(result.accessibility.nodes[0].name, 'Refresh');
    assert.equal(result.screenshot.width, 1);
    assert.deepEqual(await readFile(result.screenshot.path), png);
  } finally {
    assert.equal(path.dirname(path.resolve(root)), path.resolve(os.tmpdir()));
    await rm(root, { recursive: true, force: true });
  }
});

test('does not report success after the page reader throws', async () => {
  await assert.rejects(
    captureRenderedState({ request: async () => ({ exceptionDetails: { text: 'Exception' } }) }),
    /inspection failed/,
  );
});
