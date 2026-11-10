// @vitest-environment node
/**
 * The HTTP gateway against the real API: the panel's types held to the
 * JSON the backend actually answers, which the unit tests, answering
 * what the types say, cannot catch drifting.
 *
 * Runs against the server at `GRS_API` (`http://127.0.0.1:8080/api`, say)
 * and is skipped without one: `GRS_API=… npm run test:contract`. Every
 * runner it creates is of an owner of its own, and deleted at the end.
 */
import { afterAll, describe, expect, it } from 'vitest'

import { GatewayError } from '../core/gateway'
import type { NewRunner, Runner, RunnerImage } from '../core/runner'
import { httpGateway } from './httpGateway'

declare const process: { env: Record<string, string | undefined> }

const api = process.env.GRS_API
const owner = `contract-${Math.random().toString(36).slice(2, 10)}`

/** Fails unless `value` has exactly the shape of a `Runner`. */
function expectRunner(value: Runner) {
  expect(Object.keys(value).sort()).toEqual(
    [
      'id',
      'forge',
      'url',
      'owner',
      'repository',
      'scope',
      'name',
      'labels',
      'lifecycle',
      'cpus',
      'memory',
      'host_docker',
      'image',
      'status',
      'exit_code',
      'created_at',
    ].sort(),
  )
  expect(value.id).toMatch(/^[0-9a-f-]{36}$/)
  expect(value.forge).toBe('github')
  expect(['owner', 'repository']).toContain(value.scope)
  expect(value.repository === null || typeof value.repository === 'string').toBe(true)
  expect(value.labels.every((label) => typeof label === 'string')).toBe(true)
  expect(['persistent', 'ephemeral']).toContain(value.lifecycle)
  expect(value.cpus === null || typeof value.cpus === 'string').toBe(true)
  expect(value.memory === null || typeof value.memory === 'string').toBe(true)
  expect(typeof value.host_docker).toBe('boolean')
  expect(['created', 'running', 'restarting', 'paused', 'exited', 'dead']).toContain(value.status)
  expect(value.exit_code === null || Number.isInteger(value.exit_code)).toBe(true)
  expect(Number.isInteger(value.created_at)).toBe(true)
}

function expectImage(value: RunnerImage) {
  expect(Object.keys(value).sort()).toEqual(['description', 'forge', 'image', 'key'])
  expect(Object.values(value).every((field) => typeof field === 'string')).toBe(true)
}

describe.skipIf(api === undefined)('httpGateway against the API', () => {
  const gateway = httpGateway(api)
  const created: string[] = []

  async function create(name: string, overrides: Partial<NewRunner> = {}): Promise<Runner> {
    const runner = await gateway.createRunner({
      url: `https://github.com/${owner}/repository`,
      name,
      token: 'CONTRACTTESTTOKEN',
      labels: ['contract'],
      lifecycle: 'persistent',
      ...overrides,
    })
    created.push(runner.id)
    return runner
  }

  afterAll(async () => {
    for (const id of created) await gateway.deleteRunner(id).catch(() => {})
  })

  it('lists the images in the shape the panel reads', async () => {
    const images = await gateway.listImages()

    expect(images.length).toBeGreaterThan(0)
    images.forEach(expectImage)
  })

  it('creates a runner in the shape the panel reads, and lists it', async () => {
    const runner = await create('shape')

    expectRunner(runner)
    expect(runner).toMatchObject({ owner, repository: 'repository', scope: 'repository', labels: ['contract'] })
    const listed = await gateway.listRunners({ owner })
    listed.forEach(expectRunner)
    expect(listed.map((runner) => runner.name)).toContain('shape')
  })

  it('creates a runner serving an owner, with no repository', async () => {
    const runner = await create('owner-wide', { url: `https://github.com/${owner}`, lifecycle: 'ephemeral' })

    expect(runner).toMatchObject({ repository: null, scope: 'owner', lifecycle: 'ephemeral' })
  })

  it('creates a runner limited as asked, answering its limits as the panel reads them', async () => {
    const runner = await create('limited', { cpus: '0.5', memory: '512m' })

    expectRunner(runner)
    expect(runner).toMatchObject({ cpus: '0.5', memory: '512m' })
  })

  it('creates a runner given the host’s Docker, answering that it is', async () => {
    const given = await create('given-docker', { host_docker: true })
    const isolated = await create('isolated')

    expect(given.host_docker).toBe(true)
    expect(isolated.host_docker).toBe(false)
  })

  it('stops and starts a runner, answering it in its new state', async () => {
    const runner = await create('lifecycle')

    const stopped = await gateway.stopRunner(runner.id)
    const started = await gateway.startRunner(runner.id)

    expectRunner(stopped)
    expect(stopped.status).toBe('exited')
    expect(Number.isInteger(stopped.exit_code)).toBe(true)
    expect(['running', 'restarting']).toContain(started.status)
  })

  it('deletes a runner, which is then listed no more', async () => {
    const runner = await create('deleted')

    await expect(gateway.deleteRunner(runner.id)).resolves.toBeUndefined()

    expect((await gateway.listRunners({ owner })).map((runner) => runner.id)).not.toContain(runner.id)
  })

  it('fails with the field the API blames for a value it refuses', async () => {
    const error = await create('refused', { url: 'github.com/nowhere' }).catch((error: unknown) => error)

    expect(error).toBeInstanceOf(GatewayError)
    expect(error).toMatchObject({ status: 422, field: 'URL' })
  })

  it('fails with a conflict for a name taken in the scope', async () => {
    await create('taken')

    await expect(create('TAKEN')).rejects.toMatchObject({ status: 409, field: 'runner name' })
  })

  it('fails with not found for a runner that is not there', async () => {
    await expect(gateway.stopRunner('67e55044-10b1-426f-9247-bb680e5fe0c8')).rejects.toMatchObject({ status: 404 })
  })
})
