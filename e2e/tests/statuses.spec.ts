/**
 * What runners do on their own, shown as the panel polls: a runner that
 * cannot register, retried or not, and an ephemeral runner done with its
 * job. The stand-in image refuses the token `expired`, as GitHub refuses
 * one, and takes an ephemeral runner's job in three seconds.
 */
import { expect, test } from '@playwright/test'

import { createRunner, deleteTestRunners, owner, row } from '../support'

test.beforeEach(async ({ request }) => {
  await deleteTestRunners(request)
})

test('shows an ephemeral runner that cannot register as failed', async ({ page, request }) => {
  await createRunner(request, {
    url: `https://github.com/${owner.mine}/mindweaver`,
    name: 'expired',
    token: 'expired',
    lifecycle: 'ephemeral',
  })
  await page.goto('/')
  const expired = row(page, owner.mine, 'expired')

  await expect(expired).toContainText('Failed, exited with 1')
  await expect(expired.locator('.lamp')).toHaveClass(/lamp--down/)
  await expect(page.locator('.counts')).toContainText('failed')
})

test('shows a persistent runner that cannot register as being restarted', async ({ page, request }) => {
  await createRunner(request, {
    url: `https://github.com/${owner.mine}/mindweaver`,
    name: 'looping',
    token: 'expired',
  })
  await page.goto('/')
  const looping = row(page, owner.mine, 'looping')

  await expect(looping).toContainText('Restarting after exiting with 1', { timeout: 45_000 })
  await expect(looping.locator('.lamp')).toHaveClass(/lamp--trouble/)
})

test('shows an ephemeral runner done with its job', async ({ page, request }) => {
  await createRunner(request, {
    url: `https://github.com/${owner.mine}`,
    name: 'one-shot',
    lifecycle: 'ephemeral',
  })
  await page.goto('/')
  const oneShot = row(page, owner.mine, 'one-shot')

  await expect(oneShot).toContainText('Running')
  await expect(oneShot).toContainText('Done with its job')
  await expect(oneShot.locator('.lamp')).toHaveClass(/lamp--idle/)
})
