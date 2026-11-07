/**
 * What the end-to-end tests share: where the panel is, how to reach its
 * API and the Docker engine behind it, and how to find a runner on the
 * page.
 */
import { execFileSync } from 'node:child_process'
import { dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

import type { APIRequestContext, Locator, Page } from '@playwright/test'

export const here = dirname(fileURLToPath(import.meta.url))

/** The panel under test: one started by the tests, unless one is given. */
export const panelUrl = process.env.E2E_BASE_URL ?? 'http://127.0.0.1:18090'
/** Whether the tests started the panel, and so may restart it. */
export const ownPanel = process.env.E2E_BASE_URL === undefined

export const runnerImage = 'git-runner-swarm-e2e-runner:latest'

/**
 * The owners the tests create runners for. Every runner of an owner
 * starting `e2e-` is the tests' to delete, and no other is: the panel
 * may manage real runners on the same Docker.
 */
export const owner = { mine: 'e2e-otsobide', theirs: 'e2e-acme' } as const
const ownedByTests = (name: string) => name.startsWith('e2e-') || name.startsWith('contract-')

export interface Runner {
  id: string
  name: string
  owner: string
  status: string
  exit_code: number | null
}

export interface NewRunner {
  url: string
  name: string
  token?: string
  labels?: string[]
  lifecycle?: 'persistent' | 'ephemeral'
}

/** Creates a runner through the API, for a test that is about something else. */
export async function createRunner(request: APIRequestContext, runner: NewRunner): Promise<Runner> {
  const response = await request.post(`${panelUrl}/api/runners`, {
    data: { token: 'E2E-REGISTRATION-TOKEN', ...runner },
  })
  if (response.status() !== 201) throw new Error(`creating ${runner.name}: ${await response.text()}`)
  return (await response.json()) as Runner
}

export async function runners(request: APIRequestContext): Promise<Runner[]> {
  return (await (await request.get(`${panelUrl}/api/runners`)).json()) as Runner[]
}

export async function runnerNamed(request: APIRequestContext, name: string): Promise<Runner | undefined> {
  return (await runners(request)).find((runner) => ownedByTests(runner.owner) && runner.name === name)
}

/** Deletes every runner the tests created, and no other. */
export async function deleteTestRunners(request: APIRequestContext): Promise<void> {
  for (const runner of await runners(request)) {
    if (ownedByTests(runner.owner)) await request.delete(`${panelUrl}/api/runners/${runner.id}`)
  }
}

/** Runs the Docker CLI against the engine the panel manages. */
export function docker(...args: string[]): string {
  return execFileSync('docker', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] })
}

export function containerOf(runner: Runner): string {
  return `git-runner-swarm-${runner.id}`
}

/** A file the stand-in runner image wrote in a runner's home. */
export function fileIn(runner: Runner, file: string): string {
  return docker('exec', containerOf(runner), 'cat', `/home/runner/${file}`)
}

export async function waitForPanel(timeoutMs = 60_000): Promise<void> {
  const deadline = Date.now() + timeoutMs
  for (;;) {
    try {
      if ((await fetch(`${panelUrl}/api/health`)).ok) return
    } catch {
      // Not listening yet.
    }
    if (Date.now() > deadline) throw new Error(`the panel at ${panelUrl} did not come up`)
    await new Promise((resolve) => setTimeout(resolve, 500))
  }
}

/** The rack of an owner's runners, found by the owner's heading. */
export function rack(page: Page, name: string): Locator {
  return page.locator('section.rack', { has: page.getByRole('heading', { level: 2, name, exact: true }) })
}

/** A runner's row, found by the runner's name, within its owner's rack. */
export function row(page: Page, ownerName: string, name: string): Locator {
  return rack(page, ownerName).locator('li.row', { has: page.getByRole('heading', { level: 3, name, exact: true }) })
}
