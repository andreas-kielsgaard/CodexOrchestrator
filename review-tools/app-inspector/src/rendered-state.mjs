import { createHash } from 'node:crypto';
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

// Fixed page reader: no caller-provided script, scrolling, focus or input changes.
export function readRenderedPage() {
  const limit = (value, length = 1000) => String(value ?? '').slice(0, length);
  const selector = (element) => {
    if (!element || element.nodeType !== 1) return null;
    if (element.id && document.querySelectorAll(`#${CSS.escape(element.id)}`).length === 1)
      return `#${CSS.escape(element.id)}`;
    const parts = [];
    for (let node = element; node && node.nodeType === 1; node = node.parentElement) {
      if (node.id && document.querySelectorAll(`#${CSS.escape(node.id)}`).length === 1) {
        parts.unshift(`#${CSS.escape(node.id)}`);
        break;
      }
      const siblings = node.parentElement
        ? [...node.parentElement.children].filter((e) => e.localName === node.localName)
        : [node];
      parts.unshift(`${node.localName}:nth-of-type(${siblings.indexOf(node) + 1})`);
    }
    return parts.join(' > ');
  };
  const rect = (element) => {
    const r = element.getBoundingClientRect();
    return { x: r.x, y: r.y, width: r.width, height: r.height };
  };
  const visible = (element) =>
    element.getClientRects().length > 0 &&
    !element.closest('[hidden]') &&
    !['hidden', 'collapse'].includes(getComputedStyle(element).visibility);
  const referenced = (element, attribute) =>
    (element.getAttribute(attribute) ?? '')
      .split(/\s+/)
      .map((id) => document.getElementById(id)?.textContent ?? '')
      .join(' ')
      .trim();
  const labelText = (label) => {
    const copy = label.cloneNode(true);
    copy.querySelectorAll('input,textarea,select,button').forEach((e) => e.remove());
    return copy.textContent.trim();
  };
  const name = (element) =>
    limit(
      referenced(element, 'aria-labelledby') ||
        element.getAttribute('aria-label') ||
        [...(element.labels ?? [])].map(labelText).join(' ').trim() ||
        element.innerText ||
        element.getAttribute('title'),
    );
  const modal =
    document.querySelector('dialog:modal') ??
    [...document.querySelectorAll('[role="dialog"][aria-modal="true"]')].filter(visible).at(-1);
  const controlElements = [
    ...document.querySelectorAll(
      'button, input, textarea, select, a[href], summary, [role], [tabindex], [contenteditable="true"]',
    ),
  ].filter(visible);
  const controls = controlElements.slice(0, 500).map((element) => {
    const bounds = rect(element);
    const x = bounds.x + bounds.width / 2,
      y = bounds.y + bounds.height / 2;
    const hit = document.elementFromPoint(x, y);
    const password = element.matches('input[type="password"]');
    return {
      selector: selector(element),
      tag: element.localName,
      role: element.getAttribute('role'),
      name: name(element),
      type: element.getAttribute('type'),
      bounds,
      inViewport:
        bounds.x < innerWidth &&
        bounds.y < innerHeight &&
        bounds.x + bounds.width > 0 &&
        bounds.y + bounds.height > 0,
      hitAtCenter: hit === element || element.contains(hit),
      inert: Boolean(element.closest('[inert]')),
      blockedByModal: Boolean(modal && !modal.contains(element)),
      disabled: element.matches(':disabled') || element.getAttribute('aria-disabled') === 'true',
      checked: 'checked' in element ? element.checked : element.getAttribute('aria-checked'),
      selected: element.getAttribute('aria-selected') ?? element.getAttribute('aria-pressed'),
      expanded: element.getAttribute('aria-expanded'),
      value: password ? '[redacted]' : 'value' in element ? limit(element.value) : null,
      options:
        element instanceof HTMLSelectElement
          ? [...element.options].map((o) => ({
              text: o.text,
              value: o.value,
              disabled: o.disabled,
              selected: o.selected,
            }))
          : undefined,
    };
  });
  const text = document.body?.innerText ?? '';
  const scrolling = [...document.querySelectorAll('body *')].filter(
    (element) =>
      visible(element) &&
      (element.scrollHeight > element.clientHeight + 1 ||
        element.scrollWidth > element.clientWidth + 1) &&
      /auto|scroll|hidden|clip/.test(getComputedStyle(element).overflow),
  );
  return {
    title: document.title,
    url: location.href,
    readyState: document.readyState,
    viewport: { width: innerWidth, height: innerHeight, devicePixelRatio, scrollX, scrollY },
    text: limit(text, 24000),
    textTruncated: text.length > 24000,
    focusedSelector: selector(document.activeElement),
    controls,
    controlsTruncated: controlElements.length > 500,
    headings: [...document.querySelectorAll('h1,h2,h3,h4,h5,h6')]
      .filter(visible)
      .map((e) => ({ level: e.localName, text: limit(e.innerText), bounds: rect(e) })),
    scrollRegions: scrolling.slice(0, 100).map((e) => ({
      selector: selector(e),
      bounds: rect(e),
      clientWidth: e.clientWidth,
      scrollWidth: e.scrollWidth,
      clientHeight: e.clientHeight,
      scrollHeight: e.scrollHeight,
      scrollTop: e.scrollTop,
      scrollLeft: e.scrollLeft,
      overflow: getComputedStyle(e).overflow,
    })),
    scrollRegionsTruncated: scrolling.length > 100,
  };
}

export async function captureRenderedState(protocol, screenshotPath = null) {
  const evaluated = await protocol.request('Runtime.evaluate', {
    expression: `(${readRenderedPage.toString()})()`,
    returnByValue: true,
    silent: true,
  });
  if (evaluated.exceptionDetails || !evaluated.result?.value)
    throw new Error('Rendered page inspection failed.');
  const accessibility = await protocol.request('Accessibility.getFullAXTree');
  const nodes = accessibility.nodes
    .filter((n) => !n.ignored)
    .slice(0, 1000)
    .map((n) => ({
      nodeId: n.nodeId,
      parentId: n.parentId,
      role: n.role?.value,
      name: n.name?.value,
      properties: n.properties
        ?.filter((p) =>
          ['disabled', 'focused', 'checked', 'selected', 'expanded', 'modal', 'level'].includes(
            p.name,
          ),
        )
        .map((p) => ({ name: p.name, value: p.value?.value })),
    }));
  let screenshot = null;
  if (screenshotPath) {
    const captured = await protocol.request('Page.captureScreenshot', {
      format: 'png',
      fromSurface: true,
      captureBeyondViewport: false,
    });
    const bytes = Buffer.from(captured.data, 'base64');
    if (bytes.length < 24 || bytes.subarray(0, 8).toString('hex') !== '89504e470d0a1a0a')
      throw new Error('Rendered screenshot was not a PNG.');
    await mkdir(path.dirname(screenshotPath), { recursive: true });
    await writeFile(screenshotPath, bytes);
    screenshot = {
      path: screenshotPath,
      bytes: bytes.length,
      width: bytes.readUInt32BE(16),
      height: bytes.readUInt32BE(20),
      sha256: createHash('sha256').update(bytes).digest('hex'),
    };
  }
  return {
    rendered: evaluated.result.value,
    accessibility: {
      nodes,
      truncated: accessibility.nodes.filter((n) => !n.ignored).length > 1000,
    },
    screenshot,
  };
}
