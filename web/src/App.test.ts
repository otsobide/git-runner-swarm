import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, describe, expect, it } from 'vitest'

import App from './App.vue'
import { GatewayError } from './core/gateway'
import { aRunner, FakeGateway } from './testing/fakeGateway'

const mounted: ReturnType<typeof mount>[] = []

async function app(gateway: FakeGateway) {
  const wrapper = mount(App, { props: { gateway }, attachTo: document.body })
  mounted.push(wrapper)
  await flushPromises()
  return wrapper
}

function button(wrapper: Awaited<ReturnType<typeof app>>, text: string) {
  const found = wrapper.findAll('button').find((button) => button.text() === text)
  if (found === undefined) throw new Error(`no ${text} button`)
  return found
}

afterEach(() => {
  for (const wrapper of mounted.splice(0)) wrapper.unmount()
})

describe('App', () => {
  it('racks the runners by owner', async () => {
    const wrapper = await app(
      new FakeGateway([
        aRunner({ name: 'builder', owner: 'otsobide' }),
        aRunner({ name: 'site', owner: 'acme', url: 'https://github.com/acme/site' }),
      ]),
    )

    expect(wrapper.findAll('h2').map((owner) => owner.text())).toEqual(['acme', 'otsobide'])
    expect(wrapper.find('.counts').text()).toBe('2 running')
  })

  it('invites creating the first runner when there is none', async () => {
    const wrapper = await app(new FakeGateway())

    expect(wrapper.text()).toContain('No runners yet')
    await button(wrapper, 'New runner').trigger('click')
    expect(wrapper.find('[role="dialog"]').exists()).toBe(true)
  })

  it('says when no runner matches the filters, and clears them', async () => {
    const wrapper = await app(new FakeGateway([aRunner({ status: 'running' })]))

    await wrapper.findAll('select')[2]!.setValue('dead')
    await flushPromises()
    expect(wrapper.text()).toContain('No runner matches these filters')

    await wrapper.findAll('button').filter((button) => button.text() === 'Clear filters')[0]!.trigger('click')
    await flushPromises()
    expect(wrapper.findAll('h3')).toHaveLength(1)
  })

  it('says why the runners could not be loaded', async () => {
    const gateway = new FakeGateway()
    gateway.failWith = new GatewayError('The panel’s server cannot be reached.', null, null)

    const wrapper = await app(gateway)

    expect(wrapper.find('[role="alert"]').text()).toContain('The panel’s server cannot be reached.')
    expect(wrapper.text()).not.toContain('No runners yet')
  })

  it('creates a runner, lists it and says so', async () => {
    const wrapper = await app(new FakeGateway())

    await button(wrapper, 'New runner').trigger('click')
    await wrapper.find('input[type="url"]').setValue('https://github.com/otsobide/mindweaver')
    await wrapper.find('input[type="password"]').setValue('AESOHC2S')
    await wrapper.findAll('input[type="text"]')[0]!.setValue('builder')
    await wrapper.find('form[novalidate]').trigger('submit')
    await flushPromises()

    expect(wrapper.find('[role="dialog"]').exists()).toBe(false)
    expect(wrapper.find('[role="status"]').text()).toContain('Created builder.')
    expect(wrapper.findAll('h3').map((name) => name.text())).toEqual(['builder'])
    expect(document.activeElement?.textContent?.trim()).toBe('New runner')
  })

  it('says why stopping a runner failed, until dismissed', async () => {
    const gateway = new FakeGateway([aRunner()])
    const wrapper = await app(gateway)
    gateway.failWith = new GatewayError('the container engine failed: conflict', 502, null)

    await button(wrapper, 'Stop').trigger('click')
    await flushPromises()
    expect(wrapper.find('[role="alert"]').text()).toContain('the container engine failed: conflict')

    await button(wrapper, 'Dismiss').trigger('click')
    expect(wrapper.find('[role="alert"]').exists()).toBe(false)
  })
})
