import { execFileSync } from 'node:child_process'
import { join } from 'node:path'

import { here, ownPanel, runnerImage, waitForPanel } from './support'

/**
 * Builds the stand-in runner image and starts the panel from this
 * repository's Dockerfile, unless E2E_BASE_URL names a panel already up.
 */
export default async function globalSetup(): Promise<void> {
  if (!ownPanel) {
    await waitForPanel()
    return
  }
  execFileSync('docker', ['build', '--quiet', '--tag', runnerImage, join(here, 'fake-runner')], { stdio: 'inherit' })
  execFileSync('docker', ['compose', '--file', join(here, 'compose.yaml'), 'up', '--detach', '--build'], {
    stdio: 'inherit',
  })
  await waitForPanel()
}
