import fs from 'node:fs/promises';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';

const directory = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(directory, '../../..');
const output = path.resolve(directory, '../evidence');
const dependencies =
  process.env.REVIEW_NODE_MODULES ??
  'C:/Users/user/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules';
const require = createRequire(path.join(dependencies, '__review__.cjs'));
const { chromium } = require('playwright');
await fs.mkdir(output, { recursive: true });
const server = await createServer({
  root,
  server: { host: '127.0.0.1', port: 2381, strictPort: true },
});
await server.listen();
const browser = await chromium.launch({ headless: true, channel: 'msedge' });
const page = await browser.newPage({ viewport: { width: 1280, height: 850 } });
const results = [];
const errors = [];
page.on('pageerror', (error) => errors.push(String(error)));
const url = 'http://127.0.0.1:2381/docs/regression-review/probes/browser.html';
const shot = async (name) =>
  page.screenshot({ path: path.join(output, `${name}.png`), fullPage: true });
const workflow = async () => {
  await page.goto(url);
  await page.getByLabel('Node name', { exact: true }).waitFor();
};
try {
  await workflow();
  const foreignModel = page.getByRole('checkbox', { name: 'gpt-5.6-terra', exact: true });
  results.push({
    id: 'node-allows-forbidden-model',
    reproduced: await foreignModel.isEnabled(),
    allowed: ['gpt-5.6-sol'],
  });
  await foreignModel.check();
  await page.getByRole('button', { name: 'Save draft', exact: true }).click();
  await page.waitForFunction(() => window.audit.calls.saves.length > 0);
  results.at(-1).submittedModels = await page.evaluate(
    () => window.audit.calls.saves[0].nodes[0].nodeProfile.allowedCapabilities.models,
  );
  await shot('node-forbidden-model');

  await workflow();
  await page.getByRole('checkbox', { name: 'gpt-5.6-sol', exact: true }).uncheck();
  const shownDefault = await page.getByLabel('Default model', { exact: true }).inputValue();
  await page.getByRole('button', { name: 'Save draft', exact: true }).click();
  await page.waitForFunction(() => window.audit.calls.saves.length > 0);
  const submittedNode = await page.evaluate(() => window.audit.calls.saves[0].nodes[0].nodeProfile);
  results.push({
    id: 'default-outside-allowed-models',
    reproduced: !submittedNode.allowedCapabilities.models.includes(
      submittedNode.pinnedDefaults.model,
    ),
    shownDefault,
    submittedNode,
  });

  await workflow();
  await page.getByLabel('Node name', { exact: true }).fill('Unsaved edit');
  await page
    .getByRole('button', { name: 'Other workflow Draft v1 Not active', exact: true })
    .click();
  await page
    .getByRole('button', { name: 'Review workflow Draft v1 Active v1', exact: true })
    .click();
  const afterSwitch = await page.getByLabel('Node name', { exact: true }).inputValue();
  results.push({
    id: 'draft-lost-on-switch',
    reproduced: afterSwitch === 'Plan Author',
    afterSwitch,
  });

  await page.getByLabel('Node name', { exact: true }).fill('Unsaved edit');
  await page.getByRole('button', { name: 'Activate saved draft', exact: true }).click();
  await page.waitForFunction(
    () => document.querySelector('[aria-label="Node name"]').value === 'Plan Author',
  );
  results.push({ id: 'activation-discards-edits', reproduced: true });

  await workflow();
  await page.getByRole('button', { name: 'Compile and run', exact: true }).click();
  await page.getByLabel('Instance ID', { exact: true }).fill('review-run-A');
  await page.getByRole('button', { name: 'Compile active recipe', exact: true }).click();
  await page.getByText('1 Session Event definition compiled.', { exact: true }).waitFor();
  await page.locator('.workflow-outline__row > button').first().click();
  await page.getByRole('button', { name: 'Compile and run', exact: true }).click();
  const returnedId = await page.getByLabel('Instance ID', { exact: true }).inputValue();
  results.push({
    id: 'run-id-changes-with-old-compile-count',
    reproduced:
      returnedId !== 'review-run-A' &&
      (await page.getByText('1 Session Event definition compiled.', { exact: true }).isVisible()),
    returnedId,
  });
  await shot('run-id-changed');

  await workflow();
  await page.setViewportSize({ width: 850, height: 850 });
  results.push({
    id: 'workflow-picker-hidden-at-850px',
    reproduced: !(await page.locator('.workflow-authoring-screen__recipes').isVisible()),
  });
  await shot('workflow-narrow');
  await page.setViewportSize({ width: 1280, height: 850 });

  await page.goto(`${url}?view=capabilities`);
  await page.getByLabel('Capability profile name').waitFor();
  const runtimeToggle = page.getByRole('button', { name: 'Expand Runtime profile', exact: true });
  const panel = page.locator(`#${await runtimeToggle.getAttribute('aria-controls')}`);
  const collapseState = await panel.evaluate((el) => ({
    hidden: el.hidden,
    display: getComputedStyle(el).display,
    height: el.getBoundingClientRect().height,
  }));
  results.push({
    id: 'collapsed-section-still-visible',
    reproduced:
      collapseState.hidden && collapseState.display !== 'none' && collapseState.height > 0,
    ...collapseState,
  });
  await shot('collapsed-runtime-still-visible');
  const checkbox = page.getByRole('checkbox', { name: 'high', exact: true });
  await checkbox.scrollIntoViewIfNeeded();
  const dimensions = await checkbox.evaluate((el) => {
    const label = el.closest('label');
    const text = label.querySelector('strong');
    return {
      checkboxWidth: el.getBoundingClientRect().width,
      labelWidth: label.getBoundingClientRect().width,
      textWidth: text.getBoundingClientRect().width,
      textHeight: text.getBoundingClientRect().height,
    };
  });
  results.push({
    id: 'oversized-checkbox-squeezes-label',
    reproduced: dimensions.checkboxWidth > dimensions.labelWidth / 2,
    ...dimensions,
  });
  await shot('checkbox-label-wrap');

  await page.goto(`${url}?view=session`);
  await page.getByLabel('Message', { exact: true }).fill('First ordinary Session message');
  await page.getByRole('button', { name: 'Send', exact: true }).click();
  await page.waitForFunction(() => window.audit.calls.profileLoads.length > 0);
  await page.getByLabel('Message', { exact: true }).fill('Second ordinary Session message');
  await page.getByRole('button', { name: 'Send', exact: true }).click();
  const sendError = page.getByText('Pinned Session Profile is not available for this Session.', {
    exact: true,
  });
  await sendError.waitFor();
  const sessionCalls = await page.evaluate(() => ({
    genericSends: window.audit.calls.genericSends,
    profileSends: window.audit.calls.profileSends,
    profileLoads: window.audit.calls.profileLoads,
  }));
  results.push({
    id: 'ordinary-session-second-message-blocked',
    reproduced: sessionCalls.genericSends.length === 1 && sessionCalls.profileSends.length === 0,
    ...sessionCalls,
  });
  await shot('ordinary-session-second-message-blocked');
  results.push({ id: 'page-errors', errors });
  await fs.writeFile(
    path.join(output, 'browser-results.json'),
    JSON.stringify(results, null, 2) + '\n',
  );
  console.log(JSON.stringify(results, null, 2));
} finally {
  await browser.close();
  await server.close();
}
