import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import { reactive } from 'vue'

import type { RunnerFilters } from '../core/runner'
import { aRunner } from '../testing/fakeGateway'
import FilterBar from './FilterBar.vue'

const runners = [
  aRunner({ owner: 'otsobide', repository: 'mindweaver', labels: ['gpu'] }),
  aRunner({ owner: 'acme', repository: 'site', url: 'https://github.com/acme/site', labels: [] }),
]

function bar(filters: Partial<RunnerFilters> = {}) {
  const state = reactive<Required<RunnerFilters>>({
    owner: '',
    repository: '',
    scope: '',
    status: '',
    label: '',
    q: '',
    ...filters,
  })
  return { wrapper: mount(FilterBar, { props: { filters: state, runners } }), state }
}

function options(wrapper: ReturnType<typeof bar>['wrapper'], index: number) {
  return wrapper.findAll('select')[index]!.findAll('option').map((option) => option.text())
}

describe('FilterBar', () => {
  it('offers the owners, repositories and labels runners have', () => {
    const { wrapper } = bar()

    expect(options(wrapper, 0)).toEqual(['Every owner', 'acme', 'otsobide'])
    expect(options(wrapper, 1)).toEqual(['Every repository', 'mindweaver', 'site'])
    expect(options(wrapper, 3)).toEqual(['Any label', 'gpu'])
  })

  it('offers only the repositories of the owner chosen', () => {
    const { wrapper } = bar({ owner: 'acme' })

    expect(options(wrapper, 1)).toEqual(['Every repository', 'site'])
  })

  it('changes the filters as its fields change', async () => {
    const { wrapper, state } = bar()

    await wrapper.find('input').setValue('build')
    await wrapper.findAll('select')[0]!.setValue('otsobide')
    await wrapper.findAll('select')[2]!.setValue('exited')

    expect(state).toMatchObject({ q: 'build', owner: 'otsobide', status: 'exited' })
  })

  it('offers to clear the filters only when some narrow the list', async () => {
    const { wrapper, state } = bar()
    expect(wrapper.text()).not.toContain('Clear filters')

    state.status = 'running'
    await wrapper.vm.$nextTick()
    await wrapper.findAll('button').find((button) => button.text() === 'Clear filters')!.trigger('click')

    expect(wrapper.emitted('clear')).toHaveLength(1)
  })

  it('offers no label filter when no runner has a label', () => {
    const wrapper = mount(FilterBar, {
      props: {
        filters: reactive<Required<RunnerFilters>>({ owner: '', repository: '', scope: '', status: '', label: '', q: '' }),
        runners: [aRunner({ labels: [] })],
      },
    })

    expect(wrapper.findAll('select')).toHaveLength(3)
  })
})
