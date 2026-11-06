import { defineConfig, devices } from '@playwright/test'

import { panelUrl } from './support'

/**
 * One worker: every test acts on the same panel and the same Docker
 * engine, so they run one at a time. The panel polls every five seconds,
 * so what a container does on its own (fail, finish its job) can take
 * that long to show.
 */
export default defineConfig({
  testDir: './tests',
  workers: 1,
  fullyParallel: false,
  forbidOnly: process.env.CI !== undefined,
  retries: process.env.CI !== undefined ? 1 : 0,
  timeout: 60_000,
  expect: { timeout: 20_000 },
  reporter: process.env.CI !== undefined ? [['list'], ['html', { open: 'never' }]] : 'list',
  globalSetup: './global-setup.ts',
  globalTeardown: './global-teardown.ts',
  use: {
    baseURL: panelUrl,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      // E2E_BROWSER_CHANNEL=chrome runs an installed Google Chrome rather
      // than the Chromium Playwright downloads.
      use: { ...devices['Desktop Chrome'], channel: process.env.E2E_BROWSER_CHANNEL },
    },
  ],
})
