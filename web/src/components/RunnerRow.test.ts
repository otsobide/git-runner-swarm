import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import type { Runner } from '../core/runner'
import { aRunner } from '../testing/fakeGateway'
import RunnerRow from './RunnerRow.vue'

function row(runner: Runner, busy = false) {
  return mount(RunnerRow, { props: { runner, busy } })
}

function button(wrapper: ReturnType<typeof row>, text: string) {
  const found = wrapper.findAll('button').find((button) => button.text() === text)
  if (found === undefined) throw new Error(`no ${text} button`)
  return found
}

function buttons(wrapper: ReturnType<typeof row>) {
  return wrapper.findAll('button').map((button) => button.text())
}

describe('RunnerRow', () => {
  it('shows what a runner is and how it is doing', () => {
    const wrapper = row(aRunner({ name: 'builder', labels: ['gpu', 'large'], status: 'exited', exit_code: 1 }))

    expect(wrapper.find('h3').text()).toBe('builder')
    expect(wrapper.text()).toContain('Failed, exited with 1')
    expect(wrapper.text()).toContain('mindweaver')
    expect(wrapper.findAll('.label').map((label) => label.text())).toEqual(['gpu', 'large'])
    expect(wrapper.find('.lamp').classes()).toContain('lamp--down')
  })

  it('says a runner serving an owner serves every repository', () => {
    const wrapper = row(aRunner({ repository: null, scope: 'owner', lifecycle: 'ephemeral' }))

    expect(wrapper.find('.serves').text()).toBe('Every repository, one job at a time')
  })

  it('says what a runner may use of its host, when it is limited', () => {
    const limited = row(aRunner({ cpus: '2', memory: '4g' }))
    const unlimited = row(aRunner())

    expect(limited.find('.limits').text()).toBe('Up to 2 CPUs and 4 GiB of memory')
    expect(unlimited.find('.limits').exists()).toBe(false)
  })

  it('says when a runner’s jobs use this host’s Docker', () => {
    expect(row(aRunner({ host_docker: true })).find('.engine').text()).toBe('Its jobs use this host’s Docker')
    expect(row(aRunner()).find('.engine').exists()).toBe(false)
  })

  it('offers to stop a running runner', async () => {
    const runner = aRunner({ status: 'running' })
    const wrapper = row(runner)

    expect(buttons(wrapper)).toEqual(['Stop', 'Delete'])
    await button(wrapper, 'Stop').trigger('click')
    expect(wrapper.emitted('stop')).toEqual([[runner]])
  })

  it('offers to start a stopped runner', async () => {
    const runner = aRunner({ status: 'exited', exit_code: 0 })
    const wrapper = row(runner)

    expect(buttons(wrapper)).toEqual(['Start', 'Delete'])
    await button(wrapper, 'Start').trigger('click')
    expect(wrapper.emitted('start')).toEqual([[runner]])
  })

  it('deletes a runner only once asked again', async () => {
    const runner = aRunner({ name: 'builder' })
    const wrapper = row(runner)

    await button(wrapper, 'Delete').trigger('click')
    expect(wrapper.emitted('remove')).toBeUndefined()
    expect(wrapper.text()).toContain('Delete builder?')

    await button(wrapper, 'Delete').trigger('click')
    expect(wrapper.emitted('remove')).toEqual([[runner]])
  })

  it('keeps a runner it was asked to delete when told to', async () => {
    const wrapper = row(aRunner())

    await button(wrapper, 'Delete').trigger('click')
    await button(wrapper, 'Keep').trigger('click')

    expect(buttons(wrapper)).toEqual(['Stop', 'Delete'])
    expect(wrapper.emitted('remove')).toBeUndefined()
  })

  it('offers nothing while something is being done to the runner', () => {
    const wrapper = row(aRunner(), true)

    expect(wrapper.findAll('button').every((button) => button.attributes('disabled') !== undefined)).toBe(true)
  })
})
