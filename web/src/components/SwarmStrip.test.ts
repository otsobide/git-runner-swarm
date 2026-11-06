import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import { aRunner } from '../testing/fakeGateway'
import SwarmStrip from './SwarmStrip.vue'

describe('SwarmStrip', () => {
  it('lights a lamp for every runner, the failing ones first', () => {
    const wrapper = mount(SwarmStrip, {
      props: {
        runners: [
          aRunner({ name: 'fine' }),
          aRunner({ name: 'stopped', status: 'exited', exit_code: 0 }),
          aRunner({ name: 'broken', status: 'exited', exit_code: 1 }),
          aRunner({ name: 'looping', status: 'restarting', exit_code: 1 }),
        ],
      },
    })

    expect(wrapper.findAll('li').map((lamp) => lamp.attributes('title')?.split(':')[0])).toEqual([
      'broken',
      'looping',
      'fine',
      'stopped',
    ])
  })

  it('counts the runners by how they are doing, in words', () => {
    const wrapper = mount(SwarmStrip, {
      props: { runners: [aRunner(), aRunner(), aRunner({ status: 'exited', exit_code: 1 })] },
    })

    expect(wrapper.find('.counts').text()).toBe('1 failed, 2 running')
  })

  it('says there are no runners when there are none', () => {
    const wrapper = mount(SwarmStrip, { props: { runners: [] } })

    expect(wrapper.find('.counts').text()).toBe('No runners')
    expect(wrapper.findAll('li')).toHaveLength(0)
  })
})
