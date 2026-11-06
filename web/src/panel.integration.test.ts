/**
 * The panel as a person uses it: the whole of it mounted (its state,
 * every component) over the fake gateway, and driven through what is on
 * the page alone, the way someone at the panel would.
 */
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import App from './App.vue'
import { GatewayError } from './core/gateway'
import { POLL_EVERY_MS } from './state/useRunners'
import { aRunner, FakeGateway } from './testing/fakeGateway'

let panel: VueWrapper | null = null

async function open(gateway: FakeGateway) {
  panel = mount(App, { props: { gateway }, attachTo: document.body })
  await flushPromises()
  return panel
}

afterEach(() => {
  panel?.unmount()
  panel = null
})

function clickButton(wrapper: VueWrapper, text: string, within = wrapper.element) {
  const button = [...within.querySelectorAll('button')].find((button) => button.textContent?.trim() === text)
  if (button === undefined) throw new Error(`no ${text} button`)
  button.click()
  return flushPromises()
}

function rowOf(wrapper: VueWrapper, name: string): HTMLElement {
  const row = wrapper.findAll('li.row').find((row) => row.find('h3').text() === name)
  if (row === undefined) throw new Error(`no row for ${name}`)
  return row.element as HTMLElement
}

function racks(wrapper: VueWrapper) {
  return wrapper.findAll('section.rack').map((rack) => ({
    owner: rack.find('h2').text(),
    runners: rack.findAll('h3').map((name) => name.text()),
  }))
}

function select(wrapper: VueWrapper, name: string) {
  const found = wrapper.findAll('select').find((select) => select.element.closest('label')?.textContent?.includes(name))
  if (found === undefined) throw new Error(`no ${name} select`)
  return found
}

const fleet = () =>
  new FakeGateway([
    aRunner({ name: 'builder', owner: 'otsobide', repository: 'mindweaver', labels: ['gpu'] }),
    aRunner({ name: 'shared', owner: 'otsobide', repository: null, scope: 'owner', url: 'https://github.com/otsobide' }),
    aRunner({ name: 'deployer', owner: 'acme', repository: 'site', url: 'https://github.com/acme/site' }),
  ])

describe('the panel', () => {
  it('stops a runner from its row, then starts it again', async () => {
    const wrapper = await open(fleet())

    await clickButton(wrapper, 'Stop', rowOf(wrapper, 'builder'))
    expect(rowOf(wrapper, 'builder').textContent).toContain('Stopped')
    expect(rowOf(wrapper, 'builder').querySelector('.lamp')?.classList).toContain('lamp--idle')
    expect(wrapper.find('.counts').text()).toBe('2 running, 1 stopped')

    await clickButton(wrapper, 'Start', rowOf(wrapper, 'builder'))
    expect(rowOf(wrapper, 'builder').textContent).toContain('Running')
  })

  it('deletes a runner once asked again, and its rack with its last runner', async () => {
    const wrapper = await open(fleet())

    await clickButton(wrapper, 'Delete', rowOf(wrapper, 'deployer'))
    expect(rowOf(wrapper, 'deployer').textContent).toContain('Delete deployer?')
    await clickButton(wrapper, 'Delete', rowOf(wrapper, 'deployer'))

    expect(racks(wrapper)).toEqual([{ owner: 'otsobide', runners: ['builder', 'shared'] }])
  })

  it('narrows the racks to one owner, offering only that owner’s repositories', async () => {
    const wrapper = await open(fleet())

    await select(wrapper, 'Owner').setValue('acme')
    await flushPromises()

    expect(racks(wrapper)).toEqual([{ owner: 'acme', runners: ['deployer'] }])
    expect(select(wrapper, 'Repository').findAll('option').map((option) => option.text())).toEqual([
      'Every repository',
      'site',
    ])
    expect(select(wrapper, 'Owner').findAll('option')).toHaveLength(3)
  })

  it('clears the filters and shows every rack again', async () => {
    const wrapper = await open(fleet())
    await select(wrapper, 'Owner').setValue('acme')
    await flushPromises()

    await clickButton(wrapper, 'Clear filters')

    expect(racks(wrapper).map((rack) => rack.owner)).toEqual(['acme', 'otsobide'])
    expect((select(wrapper, 'Owner').element as HTMLSelectElement).value).toBe('')
  })

  it('creates a runner, which joins its owner’s rack', async () => {
    const wrapper = await open(fleet())

    await clickButton(wrapper, 'New runner')
    await wrapper.find('input[type="url"]').setValue('https://github.com/acme/site')
    await wrapper.find('input[type="password"]').setValue('AESOHC2S')
    await wrapper.findAll('[role="dialog"] input[type="text"]')[0]!.setValue('tester')
    await wrapper.find('[role="dialog"] form').trigger('submit')
    await flushPromises()

    expect(wrapper.find('[role="dialog"]').exists()).toBe(false)
    expect(racks(wrapper)[0]).toEqual({ owner: 'acme', runners: ['tester', 'deployer'] })
  })

  it('keeps what was typed when the API refuses a value, and points at it', async () => {
    const gateway = fleet()
    const wrapper = await open(gateway)
    await clickButton(wrapper, 'New runner')
    await wrapper.find('input[type="url"]').setValue('https://github.com/otsobide/mindweaver')
    await wrapper.find('input[type="password"]').setValue('AESOHC2S')
    await wrapper.findAll('[role="dialog"] input[type="text"]')[0]!.setValue('builder')
    gateway.failWith = new GatewayError(
      'a runner named builder already serves https://github.com/otsobide/mindweaver',
      409,
      'runner name',
    )

    await wrapper.find('[role="dialog"] form').trigger('submit')
    await flushPromises()

    expect(wrapper.find('[role="dialog"]').exists()).toBe(true)
    expect((wrapper.find('input[type="password"]').element as HTMLInputElement).value).toBe('AESOHC2S')
    expect(wrapper.find('[aria-invalid="true"]').element).toBe(
      wrapper.findAll('[role="dialog"] input[type="text"]')[0]!.element,
    )
  })

  it('closes the new runner panel on Escape, handing focus back', async () => {
    const wrapper = await open(fleet())
    await clickButton(wrapper, 'New runner')

    await wrapper.find('[role="dialog"]').trigger('keydown', { key: 'Escape' })
    await flushPromises()

    expect(wrapper.find('[role="dialog"]').exists()).toBe(false)
    expect(document.activeElement?.textContent?.trim()).toBe('New runner')
  })

  describe('kept current', () => {
    beforeEach(() => vi.useFakeTimers())
    afterEach(() => vi.useRealTimers())

    it('shows a runner that failed since it was loaded, without a reload', async () => {
      const gateway = fleet()
      const wrapper = await open(gateway)
      expect(wrapper.find('.counts').text()).toBe('3 running')

      Object.assign(gateway.runners[0]!, { status: 'restarting', exit_code: 1 })
      await vi.advanceTimersByTimeAsync(POLL_EVERY_MS)

      expect(rowOf(wrapper, 'builder').textContent).toContain('Restarting after exiting with 1')
      expect(rowOf(wrapper, 'builder').querySelector('.lamp')?.classList).toContain('lamp--trouble')
      expect(wrapper.find('.counts').text()).toBe('1 restarting, 2 running')
    })

    it('shows the server down, then the runners again once it is back', async () => {
      const gateway = fleet()
      const wrapper = await open(gateway)

      gateway.failWith = new GatewayError('The panel’s server cannot be reached.', null, null)
      await vi.advanceTimersByTimeAsync(POLL_EVERY_MS)
      expect(wrapper.find('[role="alert"]').text()).toContain('cannot be reached')
      expect(racks(wrapper)).toHaveLength(2)

      await vi.advanceTimersByTimeAsync(POLL_EVERY_MS)
      expect(wrapper.find('[role="alert"]').exists()).toBe(false)
    })
  })
})
