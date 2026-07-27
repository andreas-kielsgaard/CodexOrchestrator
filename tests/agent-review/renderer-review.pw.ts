import { expect, test } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

const evidenceDirectory = path.resolve('.dev/agent-review/renderer/recorded-plan-builder');
const labEvidenceDirectory = path.resolve('.dev/agent-review/renderer/agent-review-lab');

test('records deterministic Plan Builder review evidence', async ({ browser, context, page }) => {
  await mkdir(evidenceDirectory, { recursive: true });
  await context.tracing.start({ screenshots: true, snapshots: true, sources: true });

  const consoleEntries: Array<{
    type: string;
    text: string;
    location: { url: string; lineNumber: number; columnNumber: number };
  }> = [];
  const pageErrors: string[] = [];
  const requestFailures: Array<{ url: string; error: string }> = [];
  const httpErrors: Array<{ url: string; status: number }> = [];
  const actions: string[] = [];
  const assertions: string[] = [];

  page.on('console', (message) => {
    consoleEntries.push({
      type: message.type(),
      text: message.text(),
      location: message.location(),
    });
  });
  page.on('pageerror', (error) => {
    pageErrors.push(error.message);
  });
  page.on('requestfailed', (request) => {
    requestFailures.push({
      url: request.url(),
      error: request.failure()?.errorText ?? 'unknown request failure',
    });
  });
  page.on('response', (response) => {
    if (response.status() >= 400) {
      httpErrors.push({ url: response.url(), status: response.status() });
    }
  });

  await page.goto('/?recorded-plan-builder', { waitUntil: 'domcontentloaded' });
  actions.push('Opened the development-only recorded Plan Builder composition.');
  await expect(page.getByRole('navigation', { name: 'Application surfaces' })).toBeVisible();
  assertions.push('The normal application surface navigation rendered.');

  await page.getByRole('button', { name: 'Plan an Epic' }).click();
  actions.push('Opened Plan an Epic through the visible orchestration action.');

  const workspace = page.getByRole('main', { name: 'Plan an Epic' });
  const conversation = page.getByLabel('Epic Plan Builder conversation');
  const proposal = page.locator('.epic-plan-builder__proposal');
  await expect(workspace).toBeVisible();
  await expect(conversation).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Proposed Epic plan' })).toBeVisible();
  assertions.push('The Plan Builder workspace, conversation, and proposal rail are visible.');

  const firstSprint = page.getByRole('button', { name: /^Sprint 1 / });
  await expect(firstSprint).toHaveAttribute('aria-expanded', 'true');
  await firstSprint.click();
  actions.push('Collapsed the first proposed Sprint through its semantic button.');
  await expect(firstSprint).toHaveAttribute('aria-expanded', 'false');
  await firstSprint.click();
  actions.push('Expanded the first proposed Sprint again.');
  await expect(firstSprint).toHaveAttribute('aria-expanded', 'true');
  assertions.push('The first proposed Sprint collapses and expands through one semantic control.');

  const fullScreenshot = path.join(evidenceDirectory, 'plan-builder-1920x1080.png');
  const proposalScreenshot = path.join(evidenceDirectory, 'proposal-rail.png');
  await page.screenshot({ path: fullScreenshot, fullPage: true });
  await proposal.screenshot({ path: proposalScreenshot });
  actions.push('Captured the full page and focused proposal rail.');

  const ariaSnapshot = await workspace.ariaSnapshot();
  await writeFile(path.join(evidenceDirectory, 'semantic-snapshot.yml'), ariaSnapshot, 'utf8');

  const layout = await page.evaluate(() => {
    const describe = (selector: string) => {
      const element = document.querySelector<HTMLElement>(selector);
      if (!element) return null;
      const bounds = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return {
        x: Math.round(bounds.x),
        y: Math.round(bounds.y),
        width: Math.round(bounds.width),
        height: Math.round(bounds.height),
        clientHeight: element.clientHeight,
        scrollHeight: element.scrollHeight,
        overflowY: style.overflowY,
      };
    };
    return {
      controls: describe('.epic-plan-builder__controls'),
      conversation: describe('.epic-plan-builder__conversation'),
      proposal: describe('.epic-plan-builder__proposal'),
      proposalBody: describe('.epic-plan-builder__proposal-body'),
    };
  });

  await context.tracing.stop({ path: path.join(evidenceDirectory, 'trace.zip') });

  const revision = git('rev-parse', 'HEAD');
  const branch = git('branch', '--show-current');
  const manifest = {
    schemaVersion: 1,
    capturedAt: new Date().toISOString(),
    repository: {
      revision,
      branch,
      worktree: process.cwd(),
      workingTreeChanges: git('status', '--short').split(/\r?\n/).filter(Boolean),
    },
    application: {
      mode: 'Vite development server with recorded Plan Builder composition',
      route: '/?recorded-plan-builder',
      productionAuthority: false,
    },
    driver: {
      name: 'Playwright Test',
      version: '1.61.1',
      browser: 'Microsoft Edge',
      browserVersion: browser.version(),
    },
    platform: {
      os: `${os.type()} ${os.release()}`,
      architecture: os.arch(),
      viewport: { width: 1920, height: 1080 },
    },
    scenario: {
      id: 'recorded-plan-builder',
      startingState: 'Recorded orchestration overview with effect-limited development clients.',
    },
    actions,
    assertions,
    observations: {
      layout,
      consoleEntries,
      pageErrors,
      requestFailures,
      httpErrors,
    },
    files: [
      'plan-builder-1920x1080.png',
      'proposal-rail.png',
      'semantic-snapshot.yml',
      'trace.zip',
      'manifest.json',
    ],
    disposition: 'user-review-required',
    unverifiedClaims: [
      'The screenshots require visual review before layout fidelity is accepted.',
      'This renderer run does not verify a Tauri window, native IPC, or production behavior.',
    ],
  };
  await writeFile(
    path.join(evidenceDirectory, 'manifest.json'),
    `${JSON.stringify(manifest, null, 2)}\n`,
    'utf8',
  );

  expect(pageErrors).toEqual([]);
  expect(requestFailures).toEqual([]);
});

test('records the development Agent Review tab and its truthful lane boundaries', async ({
  browser,
  context,
  page,
}) => {
  await mkdir(labEvidenceDirectory, { recursive: true });
  await context.tracing.start({ screenshots: true, snapshots: true, sources: true });

  const pageErrors: string[] = [];
  const requestFailures: Array<{ url: string; error: string }> = [];
  page.on('pageerror', (error) => pageErrors.push(error.message));
  page.on('requestfailed', (request) => {
    requestFailures.push({
      url: request.url(),
      error: request.failure()?.errorText ?? 'unknown request failure',
    });
  });

  await page.goto('/?agent-review', { waitUntil: 'domcontentloaded' });
  const lab = page.getByRole('main', { name: 'Agent Review Lab' });
  await expect(lab).toBeVisible();
  await expect(page.getByRole('button', { name: 'Agent Review' })).toHaveAttribute(
    'aria-current',
    'page',
  );
  await expect(page.getByRole('heading', { name: 'Deterministic renderer' })).toBeVisible();
  await expect(
    page.getByRole('heading', { name: 'Windows Tauri / WebView2 attachment' }),
  ).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Native Tauri E2E' })).toBeVisible();
  await expect(
    page.getByText('Chrome DevTools MCP was not invoked', { exact: false }),
  ).toBeVisible();
  await expect(page.getByRole('button', { name: /run|attach/i })).toHaveCount(0);
  const handoff = page.getByRole('heading', { name: 'Owned instance → bounded evidence' });
  await expect(handoff).toBeAttached();
  await expect(
    page.getByText('Interface defined · application integration unproven'),
  ).toBeAttached();

  const nativeLane = page
    .getByRole('heading', { name: 'Native Tauri E2E' })
    .locator('xpath=ancestor::article');
  await nativeLane.getByText('Reproduce and inspect evidence').click();
  await expect(nativeLane.getByRole('heading', { name: 'Not established' })).toBeVisible();

  await handoff.evaluate((element) => element.scrollIntoView({ block: 'start' }));
  await page.screenshot({
    path: path.join(labEvidenceDirectory, 'agent-review-lab-evidence-1920x1080.png'),
  });
  await lab.evaluate((element) => element.scrollTo({ top: 0 }));
  await page.screenshot({
    path: path.join(labEvidenceDirectory, 'agent-review-lab-1920x1080.png'),
  });
  await writeFile(
    path.join(labEvidenceDirectory, 'semantic-snapshot.yml'),
    await lab.ariaSnapshot(),
    'utf8',
  );
  await context.tracing.stop({ path: path.join(labEvidenceDirectory, 'trace.zip') });

  const manifest = {
    schemaVersion: 1,
    capturedAt: new Date().toISOString(),
    repository: {
      revision: git('rev-parse', 'HEAD'),
      branch: git('branch', '--show-current'),
      worktree: process.cwd(),
      workingTreeChanges: git('status', '--short').split(/\r?\n/).filter(Boolean),
    },
    application: {
      mode: 'Vite development server with effect-limited recorded composition',
      route: '/?agent-review',
      productionAuthority: false,
    },
    driver: {
      name: 'Playwright Test',
      version: '1.61.1',
      browser: 'Microsoft Edge',
      browserVersion: browser.version(),
    },
    platform: {
      os: `${os.type()} ${os.release()}`,
      architecture: os.arch(),
      viewport: { width: 1920, height: 1080 },
    },
    scenario: {
      id: 'agent-review-lab',
      startingState: 'Development-only Agent Review surface selected through the URL.',
      actions: [
        'Opened the native-lane evidence disclosure.',
        'Inspected the worktree-runtime convergence boundary.',
      ],
      assertions: [
        'All three review lanes are present.',
        'The proven CDP route remains distinct from the uninvoked Chrome DevTools MCP route.',
        'No synthetic Run or Attach control grants authority.',
        'The worktree-runtime application integration is explicitly labeled unproven.',
      ],
    },
    files: [
      'agent-review-lab-1920x1080.png',
      'agent-review-lab-evidence-1920x1080.png',
      'semantic-snapshot.yml',
      'trace.zip',
      'manifest.json',
    ],
    observations: { pageErrors, requestFailures },
    disposition: 'user-review-required',
    unverifiedClaims: [
      'The screenshot requires review before visual quality is accepted.',
      'This renderer run does not establish the native lane claims displayed by the tab.',
      'The development tab is not evidence of production composition or authority.',
    ],
  };
  await writeFile(
    path.join(labEvidenceDirectory, 'manifest.json'),
    `${JSON.stringify(manifest, null, 2)}\n`,
    'utf8',
  );

  expect(pageErrors).toEqual([]);
  expect(requestFailures).toEqual([]);
});

function git(...arguments_: string[]): string {
  return execFileSync('git', arguments_, { encoding: 'utf8' }).trim();
}
