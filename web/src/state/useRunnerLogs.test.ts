import { flushPromises } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { effectScope } from 'vue'

import { GatewayError } from '../core/gateway'
import { aRunner, FakeGateway } from '../testing/fakeGateway'
import { useRunnerLogs } from './useRunnerLogs'

const runner = aRunner({ name: 'builder' })

/** A runner's log over `gateway`, read once, in a scope the test ends. */
async function opened(gateway: FakeGateway) {
  const scope = effectScope()
  const logs = scope.run(() => useRunnerLogs(gateway, runner.id, 1_000))!
  await flushPromises()
  return { logs, scope }
}

describe('useRunnerLogs', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('reads the runner’s latest lines', async () => {
    const gateway = new FakeGateway([runner])
    gateway.logs[runner.id] = [
      { stream: 'stdout', text: '√ Connected to GitHub' },
      { stream: 'stdout', text: 'Listening for Jobs' },
    ]

    const { logs, scope } = await opened(gateway)

    expect(logs.loading.value).toBe(false)
    expect(logs.lines.value.map((line) => line.text)).toEqual(['√ Connected to GitHub', 'Listening for Jobs'])
    scope.stop()
  })

  it('reads the log again a while after each answer, until it is closed', async () => {
    const gateway = new FakeGateway([runner])
    const { logs, scope } = await opened(gateway)

    gateway.logs[runner.id] = [{ stream: 'stdout', text: 'Running job: build' }]
    await vi.advanceTimersByTimeAsync(1_000)
    expect(logs.lines.value.map((line) => line.text)).toEqual(['Running job: build'])

    scope.stop()
    await vi.advanceTimersByTimeAsync(5_000)
    expect(gateway.calls.filter((call) => call.startsWith('logs'))).toHaveLength(2)
  })

  it('says why the log could not be read, and keeps trying', async () => {
    const gateway = new FakeGateway([runner])
    gateway.failWith = new GatewayError('The panel’s server cannot be reached.', null, null)

    const { logs, scope } = await opened(gateway)
    expect(logs.error.value?.message).toBe('The panel’s server cannot be reached.')

    await vi.advanceTimersByTimeAsync(1_000)
    expect(logs.error.value).toBeNull()
    scope.stop()
  })
})
