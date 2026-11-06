/**
 * Stopping, starting and deleting runners from their rows: what the page
 * shows, and what Docker then holds.
 */
import { expect, test } from '@playwright/test'

import { containerOf, createRunner, deleteTestRunners, docker, owner, row, type Runner } from '../support'

let runner: Runner

test.beforeEach(async ({ page, request }) => {
  await deleteTestRunners(request)
  runner = await createRunner(request, { url: `https://github.com/${owner.mine}/mindweaver`, name: 'builder' })
  await page.goto('/')
})

function containerState(): string {
  return docker('inspect', '--format', '{{.State.Status}}', containerOf(runner)).trim()
}

test('stops a runner from its row, and Docker stops its container', async ({ page }) => {
  const builder = row(page, owner.mine, 'builder')

  await builder.getByRole('button', { name: 'Stop' }).click()

  await expect(builder).toContainText('Stopped')
  await expect(builder.locator('.lamp')).toHaveClass(/lamp--idle/)
  await expect(builder.getByRole('button', { name: 'Start' })).toBeVisible()
  expect(containerState()).toBe('exited')
})

test('starts a stopped runner again, without registering it twice', async ({ page, request }) => {
  await request.post(`/api/runners/${runner.id}/stop`)
  await page.reload()
  const builder = row(page, owner.mine, 'builder')

  await builder.getByRole('button', { name: 'Start' }).click()

  await expect(builder).toContainText('Running')
  await expect(builder.locator('.lamp')).toHaveClass(/lamp--up/)
  expect(containerState()).toBe('running')
  const log = docker('logs', containerOf(runner))
  expect(log.match(/Runner successfully added/g)).toHaveLength(1)
  expect(log.match(/Listening for Jobs/g)).toHaveLength(2)
})

test('deletes a runner once asked again, and Docker no longer has it', async ({ page }) => {
  const builder = row(page, owner.mine, 'builder')

  await builder.getByRole('button', { name: 'Delete' }).click()
  await expect(builder).toContainText('Delete builder?')
  expect(containerState()).toBe('running')
  await builder.getByRole('button', { name: 'Delete' }).click()

  await expect(builder).toHaveCount(0)
  expect(() => containerState()).toThrow()
})

test('keeps a runner it was asked to delete when told to', async ({ page }) => {
  const builder = row(page, owner.mine, 'builder')

  await builder.getByRole('button', { name: 'Delete' }).click()
  await builder.getByRole('button', { name: 'Keep' }).click()

  await expect(builder.getByRole('button', { name: 'Stop' })).toBeVisible()
  expect(containerState()).toBe('running')
})
