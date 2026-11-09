/**
 * The screenshots the documentation shows, taken in dark mode, into
 * docs/images. Run with `npm run screenshots`, then look them over before
 * committing them.
 */
import { join } from 'node:path'

import { expect, type Page, test } from '@playwright/test'

import { here } from '../support'
import { sampleImages, sampleRunners } from './sample'

const folder = join(here, '..', 'docs', 'images')

/** Answers the panel's API with the sample runners, as a panel would. */
async function showSamples(page: Page): Promise<void> {
  await page.route(
    (url) => url.pathname === '/api/runners',
    (route) => route.fulfill({ json: sampleRunners }),
  )
  await page.route(
    (url) => url.pathname === '/api/images',
    (route) => route.fulfill({ json: sampleImages }),
  )
}

async function openPanel(page: Page): Promise<void> {
  await showSamples(page)
  await page.goto('/')
  await expect(page.getByRole('heading', { name: 'quayside-dev' })).toBeVisible()
  await page.evaluate(() => document.fonts.ready)
}

test.describe('on a desktop', () => {
  test.use({ viewport: { width: 1280, height: 800 }, deviceScaleFactor: 2 })

  test('the runners', async ({ page }) => {
    await openPanel(page)
    await page.screenshot({ path: join(folder, 'runners.png'), fullPage: true, animations: 'disabled' })
  })

  test('creating a runner', async ({ page }) => {
    // Tall enough for the whole form, down to its buttons.
    await page.setViewportSize({ width: 1280, height: 940 })
    await openPanel(page)
    await page.getByRole('button', { name: 'New runner' }).click()
    const dialog = page.getByRole('dialog', { name: 'New runner' })
    await dialog.getByLabel('Repository or owner URL').fill('https://github.com/lumen-labs/inference')
    await dialog.getByLabel('Registration token').fill('SAMPLE-REGISTRATION-TOKEN')
    await dialog.getByLabel('Name', { exact: true }).fill('gpu-2')
    await dialog.getByLabel('Labels').fill('gpu, cuda')
    await dialog.getByLabel('Labels').blur()
    await page.screenshot({ path: join(folder, 'new-runner.png'), animations: 'disabled' })
  })
})

test.describe('on a phone', () => {
  test.use({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true })

  test('the runners', async ({ page }) => {
    await openPanel(page)
    await page.screenshot({ path: join(folder, 'runners-phone.png'), animations: 'disabled' })
  })
})
