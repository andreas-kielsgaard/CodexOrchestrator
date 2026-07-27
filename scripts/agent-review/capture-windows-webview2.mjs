/* global document, location, window */

import { chromium } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const options = parseArguments(process.argv.slice(2));
const activePortFile = path.join(options.userDataFolder, 'EBWebView', 'DevToolsActivePort');
const [portText] = (await readFile(activePortFile, 'utf8')).trim().split(/\r?\n/);
const debugPort = Number(portText);
if (!Number.isInteger(debugPort)) throw new Error('DevToolsActivePort did not contain a port.');

const browser = await chromium.connectOverCDP(`http://127.0.0.1:${debugPort}`);
try {
  const pages = browser.contexts().flatMap((context) => context.pages());
  if (pages.length !== 1) throw new Error(`Expected one WebView2 page, found ${pages.length}.`);

  const page = pages[0];
  const cdp = await page.context().newCDPSession(page);
  const browserVersion = await cdp.send('Browser.getVersion');
  await page.getByRole('navigation', { name: 'Application surfaces' }).waitFor();
  const target = (await cdp.send('Target.getTargetInfo')).targetInfo;
  const runtime = await page.evaluate(() => ({
    title: document.title,
    url: location.href,
    userAgent: navigator.userAgent,
    hasTauriInternals: '__TAURI_INTERNALS__' in window,
    tauriInternalKeys: Object.keys(window.__TAURI_INTERNALS__ ?? {}).sort(),
    resources: performance.getEntriesByType('resource').map((entry) => entry.name),
  }));
  if (!runtime.hasTauriInternals) {
    throw new Error('The attached page did not expose Tauri internals after application startup.');
  }

  await page.getByRole('button', { name: 'Plan an Epic' }).click();
  const workspace = page.getByRole('main', { name: 'Plan an Epic' });
  await workspace.waitFor();
  const firstSprint = page.getByRole('button', { name: /^Sprint 1 / });
  await firstSprint.click();
  const collapsed = await firstSprint.getAttribute('aria-expanded');
  await firstSprint.click();
  const expanded = await firstSprint.getAttribute('aria-expanded');
  if (collapsed !== 'false' || expanded !== 'true') {
    throw new Error('The recorded Sprint control did not complete both semantic transitions.');
  }

  await page.screenshot({ path: path.join(options.outputDirectory, 'tauri-webview2.png') });
  await writeFile(
    path.join(options.outputDirectory, 'semantic-snapshot.yml'),
    await workspace.ariaSnapshot(),
    'utf8',
  );

  const consoleMessages = await page.consoleMessages();
  const manifest = {
    schemaVersion: 1,
    capturedAt: new Date().toISOString(),
    repository: {
      revision: git('rev-parse', 'HEAD'),
      branch: git('branch', '--show-current'),
      worktree: process.cwd(),
      workingTreeChanges: git('status', '--short').split(/\r?\n/).filter(Boolean),
    },
    endpointDiscovery: {
      method: 'DevToolsActivePort in a worktree-scoped WebView2 user data folder',
      requestedPort: 0,
      observedPort: debugPort,
      userDataFolder: path.join(options.userDataFolder, 'EBWebView'),
    },
    host: {
      executable: options.hostExecutable,
      processId: options.hostProcessId,
      applicationMode: 'Tauri development shell with recorded Plan Builder composition',
    },
    security: {
      ambientCredentials: 'scrubbed before build and launch',
      scrubbedVariableCount: Number(process.env.AGENT_REVIEW_SCRUBBED_VARIABLE_COUNT ?? 0),
      retainedCredentialValues: false,
    },
    driver: {
      name: 'Playwright connectOverCDP',
      version: '1.61.1',
      browserProduct: browserVersion.product,
      protocolVersion: browserVersion.protocolVersion,
    },
    target,
    runtime,
    actions: [
      'Attached to the only WebView2 page exposed by the launched Tauri host.',
      'Opened Plan an Epic through the visible application control.',
      'Collapsed and re-expanded the first proposed Sprint.',
    ],
    assertions: {
      onePageTarget: pages.length === 1,
      title: await page.title(),
      url: page.url(),
      tauriInternalsPresent: runtime.hasTauriInternals,
      firstSprintCollapsed: collapsed === 'false',
      firstSprintExpanded: expanded === 'true',
    },
    console: consoleMessages.map((message) => ({
      type: message.type(),
      text: message.text(),
      location: message.location(),
    })),
    files: ['tauri-webview2.png', 'semantic-snapshot.yml', 'attachment-manifest.json'],
    disposition: 'accepted',
    unverifiedClaims: [
      'This Windows-specific CDP attachment does not establish macOS or Linux support.',
      'This attachment run does not establish native IPC correctness.',
      'Chrome DevTools MCP uses this documented attachment route but was not invoked.',
    ],
  };
  await writeFile(
    path.join(options.outputDirectory, 'attachment-manifest.json'),
    `${JSON.stringify(manifest, null, 2)}\n`,
    'utf8',
  );
  process.stdout.write(`${JSON.stringify(manifest, null, 2)}\n`);
} finally {
  await browser.close();
}

function git(...arguments_) {
  return execFileSync('git', arguments_, { encoding: 'utf8' }).trim();
}

function parseArguments(arguments_) {
  const values = new Map();
  for (let index = 0; index < arguments_.length; index += 2) {
    values.set(arguments_[index], arguments_[index + 1]);
  }
  const userDataFolder = values.get('--user-data-folder');
  const outputDirectory = values.get('--output-directory');
  const hostExecutable = values.get('--host-executable');
  const hostProcessId = Number(values.get('--host-process-id'));
  if (!userDataFolder || !outputDirectory || !hostExecutable || !Number.isInteger(hostProcessId)) {
    throw new Error(
      'Expected --user-data-folder, --output-directory, --host-executable, and --host-process-id.',
    );
  }
  return { userDataFolder, outputDirectory, hostExecutable, hostProcessId };
}
