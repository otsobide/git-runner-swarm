import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { GatewayError } from '../core/gateway'
import { aRunner, FakeGateway } from '../testing/fakeGateway'
import RunnerLogs from './RunnerLogs.vue'

const runner = aRunner({ name: 'builder' })

async function logsOf(gateway: FakeGateway) {
  const wrapper = mount(RunnerLogs, { props: { runner, gateway }, attachTo: document.body })
  await flushPromises()
  return wrapper
}

describe('RunnerLogs', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('shows the runner’s latest lines, oldest first, each as it was written', async () => {
    const gateway = new FakeGateway([runner])
    gateway.logs[runner.id] = [
      { stream: 'stdout', text: 'Listening for Jobs' },
      { stream: 'stderr', text: 'Http response code: NotFound' },
    ]

    const wrapper = await logsOf(gateway)

    expect(wrapper.find('h2').text()).toBe('Logs of builder')
    const lines = wrapper.findAll('.line')
    expect(lines.map((line) => line.text())).toEqual(['Listening for Jobs', 'Http response code: NotFound'])
    expect(lines[1]!.classes()).toContain('line--stderr')
    wrapper.unmount()
  })

  it('says when the runner has logged nothing yet', async () => {
    const wrapper = await logsOf(new FakeGateway([runner]))

    expect(wrapper.text()).toContain('It has logged nothing yet.')
    expect(wrapper.find('pre').exists()).toBe(false)
    wrapper.unmount()
  })

  it('says why the log cannot be read', async () => {
    const gateway = new FakeGateway([runner])
    gateway.failWith = new GatewayError(`there is no runner ${runner.id}`, 404, null)

    const wrapper = await logsOf(gateway)

    expect(wrapper.find('[role="alert"]').text()).toBe(
      `there is no runner ${runner.id} The panel keeps trying every few seconds.`,
    )
    wrapper.unmount()
  })

  it('starts at its Close button, and closes on it or on Escape', async () => {
    const wrapper = await logsOf(new FakeGateway([runner]))

    expect(document.activeElement?.textContent).toBe('Close')
    await wrapper.find('button').trigger('click')
    await wrapper.find('[role="dialog"]').trigger('keydown', { key: 'Escape' })

    expect(wrapper.emitted('close')).toHaveLength(2)
    wrapper.unmount()
  })
})
