import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import { GatewayError } from '../core/gateway'
import type { NewRunner, Runner, RunnerImage } from '../core/runner'
import { aRunner, officialImage } from '../testing/fakeGateway'
import CreateRunnerPanel from './CreateRunnerPanel.vue'

function panel(create: (runner: NewRunner) => Promise<Runner>, images: RunnerImage[] = [officialImage]) {
  return mount(CreateRunnerPanel, { props: { images, create }, attachTo: document.body })
}

async function fill(wrapper: ReturnType<typeof panel>) {
  await wrapper.find('input[type="url"]').setValue('https://github.com/otsobide/mindweaver')
  await wrapper.find('input[type="password"]').setValue('AESOHC2S')
  const [name, labels] = wrapper.findAll('input[type="text"]')
  await name!.setValue('builder')
  await labels!.setValue('gpu, large  docker')
}

describe('CreateRunnerPanel', () => {
  it('creates the runner the form describes', async () => {
    const asked: NewRunner[] = []
    const created = aRunner({ name: 'builder' })
    const wrapper = panel(async (runner) => {
      asked.push(runner)
      return created
    })

    await fill(wrapper)
    await wrapper.find('input[value="ephemeral"]').setValue(true)
    await wrapper.find('form').trigger('submit')
    await flushPromises()

    expect(asked).toEqual([
      {
        url: 'https://github.com/otsobide/mindweaver',
        token: 'AESOHC2S',
        name: 'builder',
        labels: ['gpu', 'large', 'docker'],
        lifecycle: 'ephemeral',
        image: 'github-official',
      },
    ])
    expect(wrapper.emitted('created')).toEqual([[created]])
    wrapper.unmount()
  })

  it('shows why the API refused a value under the field holding it', async () => {
    const wrapper = panel(() =>
      Promise.reject(new GatewayError('a runner named builder already serves https://github.com/otsobide/mindweaver', 409, 'runner name')),
    )

    await fill(wrapper)
    await wrapper.find('form').trigger('submit')
    await flushPromises()

    const name = wrapper.findAll('input[type="text"]')[0]!
    expect(name.attributes('aria-invalid')).toBe('true')
    expect(name.element.parentElement!.textContent).toContain(
      'A runner named builder already serves https://github.com/otsobide/mindweaver.',
    )
    expect(wrapper.emitted('created')).toBeUndefined()
    wrapper.unmount()
  })

  it('shows a refusal of the domain’s without repeating which value it was', async () => {
    const wrapper = panel(() =>
      Promise.reject(new GatewayError('invalid URL: it names no owner or repository', 422, 'URL')),
    )

    await fill(wrapper)
    await wrapper.find('form').trigger('submit')
    await flushPromises()

    expect(wrapper.find('.error').text()).toBe('It names no owner or repository.')
    wrapper.unmount()
  })

  it('shows a failure no field is to blame for above the form', async () => {
    const wrapper = panel(() =>
      Promise.reject(new GatewayError('the image ghcr.io/actions/actions-runner:latest could not be pulled', 502, null)),
    )

    await fill(wrapper)
    await wrapper.find('form').trigger('submit')
    await flushPromises()

    expect(wrapper.find('.form-error').text()).toBe(
      'The image ghcr.io/actions/actions-runner:latest could not be pulled.',
    )
    wrapper.unmount()
  })

  it('waits for the runner before it can be asked for again', async () => {
    let finish: (runner: Runner) => void = () => {}
    const wrapper = panel(() => new Promise((resolve) => (finish = resolve)))

    await fill(wrapper)
    await wrapper.find('form').trigger('submit')
    const submit = wrapper.find('button[type="submit"]')
    expect(submit.text()).toBe('Creating runner…')
    expect(submit.attributes('disabled')).toBeDefined()

    finish(aRunner())
    await flushPromises()
    expect(submit.text()).toBe('Create runner')
    wrapper.unmount()
  })

  it('offers a choice of image only when there is one to make', async () => {
    const custom: RunnerImage = { ...officialImage, key: 'github-custom', image: 'mine/runner:1', description: 'Mine' }

    expect(panel(async () => aRunner()).find('select').exists()).toBe(false)

    const wrapper = panel(async () => aRunner(), [officialImage, custom])
    await wrapper.find('select').setValue('github-custom')
    expect(wrapper.text()).toContain('Runs mine/runner:1')
    wrapper.unmount()
  })

  it('starts at the first field, and closes on Cancel or Escape', async () => {
    const wrapper = panel(async () => aRunner())

    expect(document.activeElement).toBe(wrapper.find('input[type="url"]').element)
    await wrapper.findAll('button').find((button) => button.text() === 'Cancel')!.trigger('click')
    await wrapper.find('[role="dialog"]').trigger('keydown', { key: 'Escape' })

    expect(wrapper.emitted('close')).toHaveLength(2)
    wrapper.unmount()
  })
})
