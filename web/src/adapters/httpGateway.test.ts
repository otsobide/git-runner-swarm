import { describe, expect, it } from 'vitest'

import { GatewayError } from '../core/gateway'
import type { NewRunner } from '../core/runner'
import { httpGateway } from './httpGateway'

interface Call {
  url: string
  method: string
  headers: Record<string, string>
  body: unknown
}

/** A gateway whose fetch records each call and answers `answer`. */
function gateway(answer: () => Response | Promise<Response>) {
  const calls: Call[] = []
  const fetch = async (url: string, init: RequestInit = {}) => {
    calls.push({
      url,
      method: init.method ?? 'GET',
      headers: (init.headers ?? {}) as Record<string, string>,
      body: init.body === undefined ? undefined : JSON.parse(init.body as string),
    })
    return answer()
  }
  return { gateway: httpGateway('/api', fetch), calls }
}

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  })
}

const newRunner: NewRunner = {
  url: 'https://github.com/otsobide/mindweaver',
  name: 'builder',
  token: 'AESOHC2S',
  labels: ['gpu'],
  lifecycle: 'persistent',
}

describe('httpGateway', () => {
  it('lists runners with the filters given as query parameters', async () => {
    const { gateway: api, calls } = gateway(() => json([]))

    await api.listRunners({ owner: 'otsobide', status: '', q: 'build' })
    await api.listRunners()

    expect(calls.map((call) => call.url)).toEqual([
      '/api/runners?owner=otsobide&q=build',
      '/api/runners',
    ])
  })

  it('creates a runner by posting it as JSON', async () => {
    const { gateway: api, calls } = gateway(() => json({ id: '1' }, 201))

    await api.createRunner({ ...newRunner, image: 'github-official' })

    expect(calls[0]).toEqual({
      url: '/api/runners',
      method: 'POST',
      headers: { Accept: 'application/json', 'Content-Type': 'application/json' },
      body: { ...newRunner, image: 'github-official' },
    })
  })

  it('leaves the image out when none was chosen, for the forge’s default', async () => {
    const { gateway: api, calls } = gateway(() => json({ id: '1' }, 201))

    await api.createRunner({ ...newRunner, image: '' })

    expect(calls[0]?.body).toEqual(newRunner)
  })

  it('sends the limits given, and leaves out those left blank', async () => {
    const { gateway: api, calls } = gateway(() => json({ id: '1' }, 201))

    await api.createRunner({ ...newRunner, cpus: '1.5', memory: ' ' })

    expect(calls[0]?.body).toEqual({ ...newRunner, cpus: '1.5' })
  })

  it('reads a runner’s latest log lines, as many as the panel shows unless told', async () => {
    const lines = [{ stream: 'stdout', text: 'Listening for Jobs' }]
    const { gateway: api, calls } = gateway(() => json({ lines }))

    expect(await api.runnerLogs('a/b')).toEqual(lines)
    await api.runnerLogs('a/b', 10)

    expect(calls.map((call) => call.url)).toEqual([
      '/api/runners/a%2Fb/logs?lines=200',
      '/api/runners/a%2Fb/logs?lines=10',
    ])
  })

  it('starts, stops and deletes a runner at its own path', async () => {
    const { gateway: api, calls } = gateway(() => json({ id: 'a/b' }))

    await api.startRunner('a/b')
    await api.stopRunner('a/b')

    expect(calls.map((call) => `${call.method} ${call.url}`)).toEqual([
      'POST /api/runners/a%2Fb/start',
      'POST /api/runners/a%2Fb/stop',
    ])
  })

  it('answers nothing for a runner deleted', async () => {
    const { gateway: api, calls } = gateway(() => new Response(null, { status: 204 }))

    await expect(api.deleteRunner('1')).resolves.toBeUndefined()
    expect(calls[0]?.method).toBe('DELETE')
  })

  it('fails with the API’s reason and the field it blames', async () => {
    const { gateway: api } = gateway(() =>
      json({ error: 'invalid URL: it names no owner or repository', field: 'URL' }, 422),
    )

    const error = await api.createRunner(newRunner).catch((error: unknown) => error)

    expect(error).toBeInstanceOf(GatewayError)
    expect(error).toMatchObject({
      message: 'invalid URL: it names no owner or repository',
      status: 422,
      field: 'URL',
    })
  })

  it('fails saying what the server answered when it gave no reason', async () => {
    const { gateway: api } = gateway(
      () => new Response('<html>Bad Gateway</html>', { status: 502, statusText: 'Bad Gateway' }),
    )

    await expect(api.listRunners()).rejects.toMatchObject({
      message: 'The panel’s server answered 502 Bad Gateway.',
      status: 502,
      field: null,
    })
  })

  it('fails saying the server cannot be reached when nothing answers', async () => {
    const { gateway: api } = gateway(() => Promise.reject(new TypeError('Failed to fetch')))

    await expect(api.listImages()).rejects.toMatchObject({ status: null })
  })
})
