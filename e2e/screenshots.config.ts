import { fileURLToPath } from 'node:url'

import { defineConfig, devices } from '@playwright/test'

const port = 18091

/**
 * Takes the documentation's screenshots (`npm run screenshots`). The web
 * panel is built and served as it is shipped, and its API is answered
 * with the runners in screenshots/sample.ts, so the screenshots show the
 * same runners in the same states every time, whatever this machine's
 * Docker holds, and need no Docker at all.
 */
export default defineConfig({
  testDir: './screenshots',
  workers: 1,
  reporter: 'list',
  webServer: {
    command: `npm run build && npx vite preview --host 127.0.0.1 --port ${port} --strictPort`,
    cwd: fileURLToPath(new URL('../web', import.meta.url)),
    url: `http://127.0.0.1:${port}`,
    reuseExistingServer: false,
    timeout: 120_000,
  },
  use: {
    baseURL: `http://127.0.0.1:${port}`,
  },
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        channel: process.env.E2E_BROWSER_CHANNEL,
        colorScheme: 'dark',
        reducedMotion: 'reduce',
      },
    },
  ],
})
