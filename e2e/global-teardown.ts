import { execFileSync } from 'node:child_process'
import { join } from 'node:path'

import { request } from '@playwright/test'

import { deleteTestRunners, here, ownPanel } from './support'

/**
 * Deletes the runners the tests created, whatever became of the tests,
 * then stops the panel the tests started, unless E2E_KEEP asks to keep it
 * up for a look.
 */
export default async function globalTeardown(): Promise<void> {
  const api = await request.newContext()
  await deleteTestRunners(api).catch((error: unknown) => console.warn(`could not clean up: ${String(error)}`))
  await api.dispose()

  if (ownPanel && process.env.E2E_KEEP === undefined) {
    execFileSync('docker', ['compose', '--file', join(here, 'compose.yaml'), 'down'], { stdio: 'inherit' })
  }
}
