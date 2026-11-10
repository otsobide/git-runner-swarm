import { GatewayError, type RunnersGateway } from '../core/gateway'
import {
  LOG_LINES,
  type LogLine,
  type NewRunner,
  type Runner,
  type RunnerFilters,
  type RunnerImage,
  toQuery,
} from '../core/runner'

type Fetch = (input: string, init?: RequestInit) => Promise<Response>

/**
 * The gateway over the panel's HTTP API, at `base`. `fetch` is the
 * browser's unless another is given, as tests give one.
 */
export function httpGateway(
  base = '/api',
  fetch: Fetch = (input, init) => globalThis.fetch(input, init),
): RunnersGateway {
  async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
    const headers: Record<string, string> = { Accept: 'application/json' }
    if (init.body !== undefined) headers['Content-Type'] = 'application/json'

    let response: Response
    try {
      response = await fetch(`${base}${path}`, { ...init, headers })
    } catch {
      throw new GatewayError('The panel’s server cannot be reached. Check that it is running.', null, null)
    }

    if (!response.ok) {
      const body: unknown = await response.json().catch(() => null)
      const { error, field } = isErrorBody(body) ? body : { error: undefined, field: undefined }
      const answered = [response.status, response.statusText].filter(Boolean).join(' ')
      throw new GatewayError(
        error ?? `The panel’s server answered ${answered}.`,
        response.status,
        field ?? null,
      )
    }
    return (response.status === 204 ? undefined : await response.json()) as T
  }

  const runner = (id: string) => `/runners/${encodeURIComponent(id)}`

  return {
    listRunners(filters = {}) {
      const query = toQuery(filters).toString()
      return request<Runner[]>(query === '' ? '/runners' : `/runners?${query}`)
    },

    listImages() {
      return request<RunnerImage[]>('/images')
    },

    createRunner(newRunner: NewRunner) {
      // What was left blank is left out, for the API's default.
      const { image, cpus, memory, ...rest } = newRunner
      const body = { ...rest, ...given({ cpus, memory, image }) }
      return request<Runner>('/runners', { method: 'POST', body: JSON.stringify(body) })
    },

    startRunner(id) {
      return request<Runner>(`${runner(id)}/start`, { method: 'POST' })
    },

    stopRunner(id) {
      return request<Runner>(`${runner(id)}/stop`, { method: 'POST' })
    },

    deleteRunner(id) {
      return request<void>(runner(id), { method: 'DELETE' })
    },

    async runnerLogs(id, lines = LOG_LINES) {
      const answer = await request<{ lines: LogLine[] }>(`${runner(id)}/logs?lines=${lines}`)
      return answer.lines
    },
  }
}

/** The fields given a value that is not blank. */
function given(fields: Record<string, string | undefined>): Record<string, string> {
  return Object.fromEntries(
    Object.entries(fields).filter((field): field is [string, string] => (field[1] ?? '').trim() !== ''),
  )
}

function isErrorBody(body: unknown): body is { error?: string; field?: string } {
  return typeof body === 'object' && body !== null && 'error' in body
}
