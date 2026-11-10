import { GatewayError, type RunnersGateway } from '../core/gateway'
import type { NewRunner, Runner, RunnerFilters, RunnerImage } from '../core/runner'

/** A runner as the API would answer it, with whatever differs given. */
export function aRunner(overrides: Partial<Runner> = {}): Runner {
  return {
    id: crypto.randomUUID(),
    forge: 'github',
    url: 'https://github.com/otsobide/mindweaver',
    owner: 'otsobide',
    repository: 'mindweaver',
    scope: 'repository',
    name: 'builder',
    labels: [],
    lifecycle: 'persistent',
    cpus: null,
    memory: null,
    host_docker: false,
    image: 'ghcr.io/actions/actions-runner:latest',
    status: 'running',
    exit_code: null,
    created_at: Math.floor(Date.now() / 1000),
    ...overrides,
  }
}

export const officialImage: RunnerImage = {
  key: 'github-official',
  forge: 'github',
  image: 'ghcr.io/actions/actions-runner:latest',
  description: 'GitHub’s official runner image',
}

/**
 * A gateway holding runners in memory, which records every call made to
 * it. Like the API, it answers copies, fresh each time, as parsed from a
 * response: changing `runners` is a change on the server, seen on the
 * next call. Filters match on owner and status alone, which is as much as
 * the panel's state needs of them. `failWith` makes the next call fail.
 */
export class FakeGateway implements RunnersGateway {
  runners: Runner[]
  images: RunnerImage[] = [officialImage]
  calls: string[] = []
  failWith: GatewayError | null = null

  constructor(runners: Runner[] = []) {
    this.runners = runners
  }

  async listRunners(filters: RunnerFilters = {}): Promise<Runner[]> {
    this.record(`list ${new URLSearchParams(clean(filters)).toString()}`.trim())
    return this.runners
      .filter(
        (runner) =>
          (!filters.owner || runner.owner === filters.owner) &&
          (!filters.status || runner.status === filters.status),
      )
      .map(copy)
  }

  async listImages(): Promise<RunnerImage[]> {
    this.record('images')
    return this.images
  }

  async createRunner(newRunner: NewRunner): Promise<Runner> {
    this.record(`create ${newRunner.name}`)
    const [owner = '', repository = null] = new URL(newRunner.url).pathname.split('/').filter(Boolean)
    const runner = aRunner({
      name: newRunner.name,
      url: newRunner.url,
      owner,
      repository,
      scope: repository === null ? 'owner' : 'repository',
      labels: newRunner.labels,
      lifecycle: newRunner.lifecycle,
      cpus: newRunner.cpus ?? null,
      memory: newRunner.memory ?? null,
      host_docker: newRunner.host_docker ?? false,
    })
    this.runners = [runner, ...this.runners]
    return copy(runner)
  }

  async startRunner(id: string): Promise<Runner> {
    this.record(`start ${id}`)
    return this.update(id, { status: 'running', exit_code: null })
  }

  async stopRunner(id: string): Promise<Runner> {
    this.record(`stop ${id}`)
    return this.update(id, { status: 'exited', exit_code: 0 })
  }

  async deleteRunner(id: string): Promise<void> {
    this.record(`delete ${id}`)
    this.runners = this.runners.filter((runner) => runner.id !== id)
  }

  private record(call: string) {
    this.calls.push(call)
    const failure = this.failWith
    if (failure !== null) {
      this.failWith = null
      throw failure
    }
  }

  private update(id: string, changes: Partial<Runner>): Runner {
    const runner = this.runners.find((runner) => runner.id === id)
    if (runner === undefined) throw new GatewayError(`there is no runner ${id}`, 404, null)
    Object.assign(runner, changes)
    return copy(runner)
  }
}

/** The runner as a response would carry it: through JSON, so a copy of it. */
function copy(runner: Runner): Runner {
  return JSON.parse(JSON.stringify(runner)) as Runner
}

function clean(filters: RunnerFilters): Record<string, string> {
  return Object.fromEntries(Object.entries(filters).filter(([, value]) => value))
}
