import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import App from './App.vue'

describe('App', () => {
  it('names the panel', () => {
    expect(mount(App).text()).toContain('git-runner-swarm')
  })
})
