/**
 * A runner's logs, read from its row: what the runner's container wrote,
 * kept current while they are open.
 */
import { expect, test } from '@playwright/test'

import { createRunner, deleteTestRunners, owner, panelUrl, row } from '../support'

test.beforeEach(async ({ request }) => {
  await deleteTestRunners(request)
})

test('shows what a runner logged, and keeps showing what it logs next', async ({ page, request }) => {
  const runner = await createRunner(request, { url: `https://github.com/${owner.mine}/mindweaver`, name: 'talker' })
  await page.goto('/')

  await row(page, owner.mine, 'talker').getByRole('button', { name: 'Logs' }).click()
  const logs = page.getByRole('dialog', { name: 'Logs of talker' })
  const log = logs.getByLabel('Log of talker')
  await expect(log).toContainText('Runner successfully added')
  await expect(log).toContainText('Listening for Jobs')

  await request.post(`${panelUrl}/api/runners/${runner.id}/stop`)
  await expect(log).toContainText('Runner stopped')

  await page.keyboard.press('Escape')
  await expect(logs).toBeHidden()
  await expect(row(page, owner.mine, 'talker').getByRole('button', { name: 'Logs' })).toBeFocused()
})

test('shows why a runner failed, in what it wrote to its error output', async ({ page, request }) => {
  await createRunner(request, {
    url: `https://github.com/${owner.mine}/mindweaver`,
    name: 'refused',
    token: 'expired',
    lifecycle: 'ephemeral',
  })
  await page.goto('/')
  await expect(row(page, owner.mine, 'refused')).toContainText('Failed, exited with 1')

  await row(page, owner.mine, 'refused').getByRole('button', { name: 'Logs' }).click()

  const log = page.getByRole('dialog', { name: 'Logs of refused' }).getByLabel('Log of refused')
  await expect(log.locator('.line--stderr')).toContainText('Http response code: NotFound')
})
