/**
 * Runners live in Docker, not in the panel: they keep running while the
 * panel is down, and the panel finds them again when it is back.
 */
import { execFileSync } from 'node:child_process'
import { join } from 'node:path'

import { expect, test } from '@playwright/test'

import {
  containerOf,
  createRunner,
  deleteTestRunners,
  docker,
  here,
  owner,
  ownPanel,
  row,
  waitForPanel,
} from '../support'

test.skip(!ownPanel, 'restarts the panel, so only one the tests started')

test('keeps its runners across a restart of the panel', async ({ page, request }) => {
  await deleteTestRunners(request)
  const runner = await createRunner(request, { url: `https://github.com/${owner.mine}/mindweaver`, name: 'survivor' })

  execFileSync('docker', ['compose', '--file', join(here, 'compose.yaml'), 'stop', 'panel'], { stdio: 'ignore' })
  expect(docker('inspect', '--format', '{{.State.Status}}', containerOf(runner)).trim()).toBe('running')
  execFileSync('docker', ['compose', '--file', join(here, 'compose.yaml'), 'start', 'panel'], { stdio: 'ignore' })
  await waitForPanel()

  await page.goto('/')
  await expect(row(page, owner.mine, 'survivor')).toContainText('Running')
})
