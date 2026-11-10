/**
 * Creating runners from the panel's form: what the form says is what the
 * runner registers with, inside its own container, and what the API
 * refuses is shown against the field to blame.
 */
import { expect, type Page, test } from '@playwright/test'

import {
  containerOf,
  createRunner,
  deleteTestRunners,
  docker,
  fileIn,
  owner,
  row,
  runnerNamed,
} from '../support'

test.beforeEach(async ({ request }) => {
  await deleteTestRunners(request)
})

async function openForm(page: Page) {
  await page.goto('/')
  await page.getByRole('banner').getByRole('button', { name: 'New runner' }).click()
  return page.getByRole('dialog', { name: 'New runner' })
}

/** The value following `flag` in what config.sh was called with. */
function after(args: string[], flag: string): string | undefined {
  const at = args.indexOf(flag)
  return at === -1 ? undefined : args[at + 1]
}

test('creates a runner that registers with what the form says, and runs', async ({ page, request }) => {
  const form = await openForm(page)
  await form.getByLabel('Repository or owner URL').fill(`https://github.com/${owner.mine}/mindweaver`)
  await form.getByLabel('Registration token').fill('E2E-TOKEN-FROM-THE-FORM')
  await form.getByLabel('Name', { exact: true }).fill('builder')
  await form.getByLabel('Labels', { exact: true }).fill('gpu, large')
  await form.getByRole('button', { name: 'Create runner' }).click()

  await expect(page.getByRole('status')).toContainText('Created builder.')
  const builder = row(page, owner.mine, 'builder')
  await expect(builder).toContainText('Running')
  await expect(builder).toContainText('mindweaver')
  await expect(builder.locator('.label')).toHaveText(['gpu', 'large'])

  const runner = await runnerNamed(request, 'builder')
  expect(runner).toBeDefined()
  await expect.poll(() => fileIn(runner!, 'run.env')).toContain('RUNNER_NAME=builder')
  const args = fileIn(runner!, 'config.args').trim().split('\n')
  expect(after(args, '--url')).toBe(`https://github.com/${owner.mine}/mindweaver`)
  expect(after(args, '--token')).toBe('E2E-TOKEN-FROM-THE-FORM')
  expect(after(args, '--name')).toBe('builder')
  expect(after(args, '--labels')).toBe('gpu,large')
  expect(args).toContain('--unattended')
  expect(args).not.toContain('--ephemeral')
  expect(fileIn(runner!, 'run.env')).not.toContain('E2E-TOKEN-FROM-THE-FORM')
})

test('creates a runner for every repository of an owner, taking one job', async ({ page, request }) => {
  const form = await openForm(page)
  await form.getByLabel('Repository or owner URL').fill(`https://github.com/${owner.mine}`)
  await form.getByLabel('Registration token').fill('E2E-TOKEN')
  await form.getByLabel('Name', { exact: true }).fill('one-shot')
  await form.getByRole('radio', { name: /One job/ }).check()
  await form.getByRole('button', { name: 'Create runner' }).click()

  await expect(row(page, owner.mine, 'one-shot')).toContainText('Every repository, one job at a time')
  const runner = await runnerNamed(request, 'one-shot')
  await expect.poll(() => fileIn(runner!, 'config.args')).toContain('--ephemeral')
})

test('creates a runner limited as the form says, which Docker holds it to', async ({ page, request }) => {
  const form = await openForm(page)
  await form.getByLabel('Repository or owner URL').fill(`https://github.com/${owner.mine}/mindweaver`)
  await form.getByLabel('Registration token').fill('E2E-TOKEN')
  await form.getByLabel('Name', { exact: true }).fill('limited')
  await form.getByLabel('CPUs', { exact: true }).fill('0.5')
  await form.getByLabel('Memory', { exact: true }).fill('256m')
  await form.getByRole('button', { name: 'Create runner' }).click()

  await expect(row(page, owner.mine, 'limited')).toContainText('Up to 0.5 CPUs and 256 MiB of memory')
  const runner = await runnerNamed(request, 'limited')
  const limits = docker('inspect', '--format', '{{.HostConfig.NanoCpus}} {{.HostConfig.Memory}}', containerOf(runner!))
  expect(limits.trim()).toBe(`500000000 ${256 * 1024 * 1024}`)
})

test('points at the field holding a value the API refuses', async ({ page }) => {
  const form = await openForm(page)
  await form.getByLabel('Repository or owner URL').fill(`github.com/${owner.mine}/mindweaver`)
  await form.getByLabel('Registration token').fill('E2E-TOKEN')
  await form.getByLabel('Name', { exact: true }).fill('builder')
  await form.getByRole('button', { name: 'Create runner' }).click()

  await expect(form.getByRole('alert')).toHaveText('It has no scheme, such as `https://`.')
  await expect(form.getByLabel('Repository or owner URL')).toHaveAttribute('aria-invalid', 'true')
  await expect(form.getByLabel('Registration token')).toHaveValue('E2E-TOKEN')
})

test('refuses a second runner of a name in the same scope', async ({ page, request }) => {
  await createRunner(request, { url: `https://github.com/${owner.mine}/mindweaver`, name: 'builder' })

  const form = await openForm(page)
  await form.getByLabel('Repository or owner URL').fill(`https://github.com/${owner.mine}/mindweaver`)
  await form.getByLabel('Registration token').fill('E2E-TOKEN')
  await form.getByLabel('Name', { exact: true }).fill('Builder')
  await form.getByRole('button', { name: 'Create runner' }).click()

  await expect(form.getByRole('alert')).toHaveText(
    `A runner named Builder already serves https://github.com/${owner.mine}/mindweaver.`,
  )
  await expect(form.getByLabel('Name', { exact: true })).toHaveAttribute('aria-invalid', 'true')
})

test.describe('on a phone-sized screen', () => {
  test.use({ viewport: { width: 390, height: 844 } })

  test('creates a runner and shows it', async ({ page }) => {
    const form = await openForm(page)
    await form.getByLabel('Repository or owner URL').fill(`https://github.com/${owner.mine}/mindweaver`)
    await form.getByLabel('Registration token').fill('E2E-TOKEN')
    await form.getByLabel('Name', { exact: true }).fill('pocket')
    await form.getByRole('button', { name: 'Create runner' }).click()

    const pocket = row(page, owner.mine, 'pocket')
    await expect(pocket).toContainText('Running')
    await pocket.scrollIntoViewIfNeeded()
    // Stacked to fit, its buttons whole within the screen's width.
    await expect(pocket.getByRole('button', { name: 'Stop' })).toBeInViewport({ ratio: 1 })
    await expect(pocket.getByRole('button', { name: 'Delete' })).toBeInViewport({ ratio: 1 })
  })
})
