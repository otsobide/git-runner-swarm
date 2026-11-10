import type { LogLine, NewRunner, Runner, RunnerFilters, RunnerImage } from './runner'

/**
 * Where the panel gets runners from, and sends what is done to them: the
 * port the panel's state works through. The HTTP adapter implements it
 * against the API; tests, with a fake.
 */
export interface RunnersGateway {
  listRunners(filters?: RunnerFilters): Promise<Runner[]>
  listImages(): Promise<RunnerImage[]>
  createRunner(runner: NewRunner): Promise<Runner>
  startRunner(id: string): Promise<Runner>
  stopRunner(id: string): Promise<Runner>
  deleteRunner(id: string): Promise<void>
  /** The latest lines a runner logged, oldest first, `lines` of them at most. */
  runnerLogs(id: string, lines?: number): Promise<LogLine[]>
}

/**
 * A request that failed: refused by the API, which says why and, when one
 * value is to blame, which field holds it; or never answered at all, when
 * `status` is null.
 */
export class GatewayError extends Error {
  readonly status: number | null
  readonly field: string | null

  constructor(message: string, status: number | null, field: string | null) {
    super(message)
    this.name = 'GatewayError'
    this.status = status
    this.field = field
  }
}

/** Whatever was thrown, as a GatewayError: one never answered, unless it is one. */
export function asGatewayError(error: unknown): GatewayError {
  return error instanceof GatewayError
    ? error
    : new GatewayError(error instanceof Error ? error.message : String(error), null, null)
}
