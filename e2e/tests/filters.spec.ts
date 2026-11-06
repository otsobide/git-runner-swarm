/**
 * Finding runners among many: the filters, with runners of two owners on
 * the same Docker, and only the tests' own ever looked at, as the engine
 * may hold real ones.
 */
import { expect, test } from '@playwright/test'

import { createRunner, deleteTestRunners, owner, rack, row } from '../support'

test.beforeEach(async ({ page, request }) => {
  await deleteTestRunners(request)
  await createRunner(request, {
    url: `https://github.com/${owner.mine}/mindweaver`,
    name: 'builder',
    labels: ['gpu'],
  })
  await createRunner(request, { url: `https://github.com/${owner.mine}`, name: 'shared' })
  const deployer = await createRunner(request, {
    url: `https://github.com/${owner.theirs}/website`,
    name: 'deployer',
    labels: ['deploy'],
  })
  await request.post(`/api/runners/${deployer.id}/stop`)
  await page.goto('/')
})

test('racks the runners of each owner together', async ({ page }) => {
  await expect(rack(page, owner.mine).getByRole('heading', { level: 3 })).toHaveText(['shared', 'builder'])
  await expect(rack(page, owner.mine)).toContainText('2 runners')
  await expect(rack(page, owner.theirs).getByRole('heading', { level: 3 })).toHaveText(['deployer'])
})

test('narrows the list to one owner, offering only its repositories', async ({ page }) => {
  await page.getByLabel('Owner', { exact: true }).selectOption(owner.theirs)

  await expect(rack(page, owner.theirs)).toBeVisible()
  await expect(rack(page, owner.mine)).toHaveCount(0)
  await expect(page.getByLabel('Repository', { exact: true }).locator('option')).toHaveText(['Every repository', 'website'])
})

test('narrows the list by status and by label', async ({ page }) => {
  await page.getByLabel('Status', { exact: true }).selectOption('exited')
  await expect(row(page, owner.theirs, 'deployer')).toBeVisible()
  await expect(rack(page, owner.mine)).toHaveCount(0)

  await page.getByLabel('Status', { exact: true }).selectOption('')
  await page.getByLabel('Label', { exact: true }).selectOption('gpu')
  await expect(row(page, owner.mine, 'builder')).toBeVisible()
  await expect(row(page, owner.mine, 'shared')).toHaveCount(0)
})

test('finds runners by text in their name or URL', async ({ page }) => {
  await page.getByLabel('Search runners').fill('mindweav')

  await expect(row(page, owner.mine, 'builder')).toBeVisible()
  await expect(row(page, owner.mine, 'shared')).toHaveCount(0)
  await expect(rack(page, owner.theirs)).toHaveCount(0)
})

test('says when no runner matches, and clears the filters', async ({ page }) => {
  await page.getByLabel('Search runners').fill('no-runner-is-called-this')
  await expect(page.getByRole('heading', { name: 'No runner matches these filters' })).toBeVisible()

  await page.getByRole('main').getByRole('button', { name: 'Clear filters' }).click()

  await expect(page.getByLabel('Search runners')).toHaveValue('')
  await expect(rack(page, owner.mine)).toBeVisible()
  await expect(rack(page, owner.theirs)).toBeVisible()
})
