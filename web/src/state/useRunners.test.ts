import { flushPromises } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { effectScope, nextTick } from 'vue'

import { GatewayError } from '../core/gateway'
import type { Runner } from '../core/runner'
import { aRunner, FakeGateway } from '../testing/fakeGateway'
import { useRunners } from './useRunners'

/** The panel's state over `gateway`, loaded, in a scope the test ends. */
async function loaded(gateway: FakeGateway) {
  const scope = effectScope()
  const state = scope.run(() => useRunners(gateway, 1_000))!
  await flushPromises()
  return { state, scope }
}

const names = (runners: Runner[]) => runners.map((runner) => runner.name)

describe('useRunners', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('loads the runners and the images', async () => {
    const gateway = new FakeGateway([aRunner({ name: 'builder' })])

    const { state, scope } = await loaded(gateway)

    expect(state.loading.value).toBe(false)
    expect(names(state.shown.value)).toEqual(['builder'])
    expect(state.images.value.map((image) => image.key)).toEqual(['github-official'])
    scope.stop()
  })

  it('shows the runners the filters match, and keeps every runner for the rest', async () => {
    const gateway = new FakeGateway([
      aRunner({ name: 'mine', owner: 'otsobide' }),
      aRunner({ name: 'theirs', owner: 'acme', url: 'https://github.com/acme/site' }),
    ])
    const { state, scope } = await loaded(gateway)

    state.filters.owner = 'acme'
    await flushPromises()

    expect(names(state.shown.value)).toEqual(['theirs'])
    expect(names(state.all.value)).toEqual(['mine', 'theirs'])
    expect(gateway.calls).toContain('list owner=acme')
    scope.stop()
  })

  it('asks for the runners again every so often, until its scope ends', async () => {
    const gateway = new FakeGateway()
    const { scope } = await loaded(gateway)
    const asked = () => gateway.calls.filter((call) => call === 'list').length

    await vi.advanceTimersByTimeAsync(3_000)
    expect(asked()).toBe(4)

    scope.stop()
    await vi.advanceTimersByTimeAsync(3_000)
    expect(asked()).toBe(4)
  })

  it('keeps only the latest answer when the filters change while one is on its way', async () => {
    const gateway = new FakeGateway([aRunner({ name: 'running' }), aRunner({ name: 'stopped', status: 'exited' })])
    const { state, scope } = await loaded(gateway)
    let release: () => void = () => {}
    const slow = new Promise<void>((resolve) => (release = resolve))
    const listRunners = gateway.listRunners.bind(gateway)
    gateway.listRunners = async (filters) => {
      const answer = await listRunners(filters)
      if (filters?.status === 'running') await slow
      return answer
    }

    state.filters.status = 'running'
    await nextTick()
    state.filters.status = 'exited'
    await flushPromises()
    release()
    await flushPromises()

    expect(names(state.shown.value)).toEqual(['stopped'])
    scope.stop()
  })

  it('says why the runners could not be loaded, until they can be again', async () => {
    const gateway = new FakeGateway()
    gateway.failWith = new GatewayError('The panel’s server cannot be reached.', null, null)

    const { state, scope } = await loaded(gateway)
    expect(state.loadError.value?.message).toBe('The panel’s server cannot be reached.')

    await state.refresh()
    expect(state.loadError.value).toBeNull()
    scope.stop()
  })

  it('stops, starts and deletes a runner, then asks for the runners again', async () => {
    const runner = aRunner({ name: 'builder' })
    const gateway = new FakeGateway([runner])
    const { state, scope } = await loaded(gateway)

    await state.stop(runner)
    expect(state.shown.value[0]?.status).toBe('exited')
    await state.start(runner)
    expect(state.shown.value[0]?.status).toBe('running')
    await state.remove(runner)
    expect(state.shown.value).toEqual([])

    expect(gateway.calls.filter((call) => !call.startsWith('list') && call !== 'images')).toEqual([
      `stop ${runner.id}`,
      `start ${runner.id}`,
      `delete ${runner.id}`,
    ])
    scope.stop()
  })

  it('marks a runner busy while something is done to it', async () => {
    const runner = aRunner()
    const { state, scope } = await loaded(new FakeGateway([runner]))

    const stopping = state.stop(runner)
    expect(state.busy.has(runner.id)).toBe(true)
    await stopping
    expect(state.busy.has(runner.id)).toBe(false)
    scope.stop()
  })

  it('says why something done to a runner failed, until dismissed', async () => {
    const runner = aRunner()
    const gateway = new FakeGateway([runner])
    const { state, scope } = await loaded(gateway)
    gateway.failWith = new GatewayError('the container engine failed: conflict', 502, null)

    await state.stop(runner)
    expect(state.notice.value).toBe('the container engine failed: conflict')

    state.dismissNotice()
    expect(state.notice.value).toBeNull()
    scope.stop()
  })

  it('creates a runner and shows it', async () => {
    const gateway = new FakeGateway()
    const { state, scope } = await loaded(gateway)

    await state.create({
      url: 'https://github.com/otsobide/mindweaver',
      name: 'new-one',
      token: 'AESOHC2S',
      labels: [],
      lifecycle: 'persistent',
    })

    expect(names(state.shown.value)).toEqual(['new-one'])
    scope.stop()
  })

  it('lets a runner that could not be created fail, for the form to say why', async () => {
    const gateway = new FakeGateway()
    const { state, scope } = await loaded(gateway)
    gateway.failWith = new GatewayError('a runner named builder already serves …', 409, 'runner name')

    await expect(
      state.create({ url: 'https://github.com/o/r', name: 'builder', token: 't', labels: [], lifecycle: 'persistent' }),
    ).rejects.toMatchObject({ field: 'runner name' })
    scope.stop()
  })

  it('forgets a repository chosen for an owner no longer chosen', async () => {
    const gateway = new FakeGateway([
      aRunner({ owner: 'otsobide', repository: 'mindweaver' }),
      aRunner({ owner: 'acme', repository: 'site', url: 'https://github.com/acme/site' }),
    ])
    const { state, scope } = await loaded(gateway)
    state.filters.owner = 'otsobide'
    state.filters.repository = 'mindweaver'
    await flushPromises()

    state.filters.owner = 'acme'
    await flushPromises()

    expect(state.filters.repository).toBe('')
    scope.stop()
  })

  it('clears every filter at once', async () => {
    const { state, scope } = await loaded(new FakeGateway())
    state.filters.owner = 'otsobide'
    state.filters.q = 'build'

    state.clearFilters()

    expect({ ...state.filters }).toEqual({ owner: '', repository: '', scope: '', status: '', label: '', q: '' })
    scope.stop()
  })
})
