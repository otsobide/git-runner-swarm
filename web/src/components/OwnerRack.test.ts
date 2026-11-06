import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import { groupByOwner, type Runner } from '../core/runner'
import { aRunner } from '../testing/fakeGateway'
import OwnerRack from './OwnerRack.vue'

function rack(runners: Runner[], busy: Set<string> = new Set()) {
  const [group] = groupByOwner(runners)
  return mount(OwnerRack, { props: { group: group!, busy } })
}

describe('OwnerRack', () => {
  it('names the owner and counts its runners', () => {
    const wrapper = rack([aRunner({ name: 'a' }), aRunner({ name: 'b' })])

    expect(wrapper.find('h2').text()).toBe('otsobide')
    expect(wrapper.find('.count').text()).toBe('2 runners')
    expect(wrapper.findAll('h3').map((name) => name.text())).toEqual(['a', 'b'])
  })

  it('counts a single runner as one', () => {
    expect(rack([aRunner()]).find('.count').text()).toBe('1 runner')
  })

  it('names the forge only when it is not GitHub', () => {
    expect(rack([aRunner()]).find('.host').exists()).toBe(false)
    expect(rack([aRunner({ url: 'http://gitea.lan:3000/otsobide/app' })]).find('.host').text()).toBe(
      'on gitea.lan:3000',
    )
  })

  it('passes on what is asked of its runners, and which are busy', async () => {
    const runner = aRunner()
    const wrapper = rack([runner], new Set([runner.id]))

    expect(wrapper.find('button').attributes('disabled')).toBeDefined()
    wrapper.findComponent({ name: 'RunnerRow' }).vm.$emit('stop', runner)
    expect(wrapper.emitted('stop')).toEqual([[runner]])
  })
})
