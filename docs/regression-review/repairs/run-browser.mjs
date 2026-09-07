import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';

const directory = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(directory, '../../..');
const output = path.join(directory, 'evidence');
const require = createRequire(
  process.env.REVIEW_NODE_MODULES
    ? path.join(process.env.REVIEW_NODE_MODULES, '__review__.cjs')
    : import.meta.url,
);
const { chromium } = require('playwright');
await fs.mkdir(output, { recursive: true });
const server = await createServer({
  root,
  server: { host: '127.0.0.1', port: 2382, strictPort: true },
});
await server.listen();
const browser = await chromium.launch({
  headless: true,
  channel: process.env.REVIEW_BROWSER_CHANNEL ?? 'msedge',
});
const page = await browser.newPage({ viewport: { width: 1280, height: 850 } });
const errors = [];
const results = [];
page.on('pageerror', (error) => errors.push(String(error)));
page.on('dialog', (dialog) => dialog.accept());
const shot = (name) => page.screenshot({ path: path.join(output, `${name}.png`), fullPage: true });
try {
  await page.goto('http://127.0.0.1:2382/docs/regression-review/repairs/browser.html');
  await page.getByRole('button', { name: 'Configure Author', exact: true }).waitFor();
  await shot('01-flow');
  const author = page.getByRole('button', { name: 'Configure Author', exact: true });
  const before = await author.boundingBox();
  await page.mouse.move(before.x + 80, before.y + 35);
  await page.mouse.down();
  await page.mouse.move(before.x + 140, before.y + 85, { steps: 6 });
  await page.mouse.up();
  const after = await author.boundingBox();
  assert.ok(after.x > before.x + 40 && after.y > before.y + 30, 'drag moves node');
  await page.getByRole('button', { name: 'Connect', exact: true }).click();
  await author.click();
  await page.getByRole('button', { name: 'Configure Reviewer', exact: true }).click();
  await page.getByRole('button', { name: 'Edit Author → Reviewer', exact: true }).waitFor();
  await shot('02-connection');
  await page.getByRole('button', { name: 'Close editor' }).click();
  await page.getByRole('button', { name: 'Save draft', exact: true }).click();
  await page.waitForFunction(() => window.repairs.fixture.states[0].draft.revision === 2);
  assert.equal(
    await page.evaluate(() => window.repairs.fixture.states[0].draft.connections.length),
    1,
  );
  results.push('Canvas drag, connect and save');

  for (const width of [1280, 958, 850, 640]) {
    await page.setViewportSize({ width, height: 850 });
    await author.click();
    const checkbox = page.getByRole('checkbox', { name: 'model-a', exact: true });
    const box = await checkbox.boundingBox();
    assert.ok(box.width <= 22 && box.height <= 22, `${width}: compact checkbox`);
    assert.equal(await page.getByRole('checkbox', { name: 'model-b', exact: true }).count(), 0);
    const toggle = page.getByRole('button', { name: /Exposed capabilities/ });
    await toggle.click();
    assert.equal(await checkbox.isVisible(), false, `${width}: collapsed fields hidden`);
    const content = page.locator('.collapsible-section__content[hidden]');
    assert.ok((await content.count()) > 0);
    assert.ok(
      (
        await content.evaluateAll((items) =>
          items.map((item) => item.getBoundingClientRect().height),
        )
      ).every((height) => height === 0),
    );
    await shot(`03-node-${width}`);
    await page.getByRole('button', { name: 'Close editor' }).click();
    assert.ok(await page.getByLabel('New Workflow name').isVisible());
    const create = page.getByRole('button', { name: 'Create instance', exact: true });
    await create.click();
    assert.ok(await page.getByRole('dialog', { name: 'Create Workflow instance' }).isVisible());
    await shot(`04-create-${width}`);
    await page.getByRole('button', { name: 'Close instance creation' }).click();
    results.push(`${width}px: collapse, checkbox, workflow picker and instance dialog`);
  }
  await page.setViewportSize({ width: 1280, height: 850 });
  await page.getByRole('button', { name: 'Create instance', exact: true }).click();
  await page.getByLabel('Instance name', { exact: true }).fill('Review run');
  await page.getByRole('button', { name: 'Use demo worktree' }).click();
  await page
    .getByRole('dialog')
    .getByRole('button', { name: 'Create instance', exact: true })
    .click();
  await page.getByRole('heading', { name: 'Review run', exact: true }).waitFor();
  assert.equal(
    await page.evaluate(() => window.repairs.calls.messages),
    0,
    'creating instance must not send',
  );
  const id = await page.evaluate(() => window.repairs.instances[0].id);
  await shot('05-saved-instance');
  await page.getByLabel('Request', { exact: true }).fill('Review this plan.');
  await page.getByRole('button', { name: 'Send request', exact: true }).click();
  await page.getByRole('button', { name: 'Edit identity', exact: true }).waitFor();
  await shot('06-instance-session');
  await page.getByRole('button', { name: 'Back', exact: true }).click();
  await page.getByRole('button', { name: /Review run Review workflow/ }).click();
  await page.getByRole('heading', { name: 'Review run', exact: true }).waitFor();
  assert.equal(await page.evaluate(() => window.repairs.instances[0].id), id);
  results.push('Stored instance creation, node request, Session pane, Back and reopen');
  await page.reload();
  await page.getByRole('button', { name: /Review run Review workflow/ }).click();
  await page.getByRole('heading', { name: 'Review run', exact: true }).waitFor();
  assert.equal(await page.evaluate(() => window.repairs.instances[0].id), id);
  results.push('Reload and reopen saved instance (browser fixture storage)');
  assert.deepEqual(errors, []);
  await fs.writeFile(
    path.join(output, 'results.json'),
    JSON.stringify(
      {
        results,
        errors,
        evidence: 'Real components with fake clients, not a native or provider run.',
      },
      null,
      2,
    ),
  );
  console.log(JSON.stringify(results, null, 2));
} catch (error) {
  await shot('failure');
  console.error(errors);
  throw error;
} finally {
  await browser.close();
  await server.close();
}
