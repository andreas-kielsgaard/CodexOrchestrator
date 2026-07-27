import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests/agent-review',
  testMatch: 'renderer-review.pw.ts',
  fullyParallel: false,
  workers: 1,
  timeout: 60_000,
  reporter: 'list',
  outputDir: '.dev/agent-review/playwright/test-results',
  use: {
    baseURL: 'http://127.0.0.1:1437',
    browserName: 'chromium',
    channel: 'msedge',
    headless: true,
    viewport: { width: 1920, height: 1080 },
  },
  webServer: {
    command: 'npm run dev -- --host 127.0.0.1 --port 1437',
    url: 'http://127.0.0.1:1437',
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
