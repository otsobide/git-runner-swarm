/**
 * Runners as the panel shows them: the API's types, and what a person
 * looking at a runner needs to know of it, worked out from them.
 */

export type Forge = 'github'
export type RunnerStatus = 'created' | 'running' | 'restarting' | 'paused' | 'exited' | 'dead'
export type ScopeKind = 'owner' | 'repository'
export type Lifecycle = 'persistent' | 'ephemeral'

/** A runner, as the API answers it. */
export interface Runner {
  id: string
  forge: Forge
  /** The URL the runner registers with. */
  url: string
  owner: string
  repository: string | null
  scope: ScopeKind
  name: string
  labels: string[]
  lifecycle: Lifecycle
  /** The most CPUs it may use, `1.5`; `null` for no limit. */
  cpus: string | null
  /** The most memory it may use, as Docker writes sizes, `4g`; `null` for no limit. */
  memory: string | null
  /** Whether its jobs are given the host's Docker. */
  host_docker: boolean
  /** Whether its jobs are given the host's GPUs. */
  host_gpu: boolean
  image: string
  status: RunnerStatus
  exit_code: number | null
  /** Seconds since the Unix epoch. */
  created_at: number
}

/** One line a runner logged, without its line break, as the API answers it. */
export interface LogLine {
  /** The output it was written to. */
  stream: 'stdout' | 'stderr'
  text: string
}

/** How many of a runner's latest log lines the panel shows. */
export const LOG_LINES = 200

/** A runner image runners can be created from. */
export interface RunnerImage {
  key: string
  forge: Forge
  image: string
  description: string
}

/** What a runner is to be created as, and the token it registers with. */
export interface NewRunner {
  url: string
  name: string
  token: string
  labels: string[]
  lifecycle: Lifecycle
  /** The most CPUs it may use, `1.5`; no limit when left out. */
  cpus?: string
  /** The most memory it may use, `4g`; no limit when left out. */
  memory?: string
  /** Whether its jobs are given the host's Docker; not when left out. */
  host_docker?: boolean
  /** Whether its jobs are given the host's GPUs; not when left out. */
  host_gpu?: boolean
  /** The key of the image; the forge's default when left out. */
  image?: string
}

/** Which runners to list. A criterion left empty narrows nothing. */
export interface RunnerFilters {
  owner?: string
  repository?: string
  scope?: ScopeKind | ''
  status?: RunnerStatus | ''
  label?: string
  q?: string
}

/**
 * How a runner is doing, as a person scanning the panel needs to know at a
 * glance: taking jobs (`up`), failing and being retried (`trouble`),
 * failed (`down`), or stopped as it was meant to be (`idle`).
 */
export type Health = 'up' | 'trouble' | 'down' | 'idle'

export function healthOf(runner: Runner): Health {
  switch (runner.status) {
    case 'running':
      return 'up'
    case 'restarting':
      return 'trouble'
    case 'dead':
      return 'down'
    case 'exited':
      return runner.exit_code === null || runner.exit_code === 0 ? 'idle' : 'down'
    case 'created':
    case 'paused':
      return 'idle'
  }
}

/** What a runner is doing, in a few words. */
export function statusText(runner: Runner): string {
  const code = runner.exit_code
  switch (runner.status) {
    case 'running':
      return 'Running'
    case 'restarting':
      return code !== null && code !== 0 ? `Restarting after exiting with ${code}` : 'Restarting'
    case 'exited':
      if (code !== null && code !== 0) return `Failed, exited with ${code}`
      return runner.lifecycle === 'ephemeral' ? 'Done with its job' : 'Stopped'
    case 'created':
      return 'Created, not started'
    case 'paused':
      return 'Paused'
    case 'dead':
      return 'Dead'
  }
}

/** Whether a runner can be started: it is not running, nor being restarted. */
export function canStart(runner: Runner): boolean {
  return runner.status === 'exited' || runner.status === 'created'
}

/** Whether a runner can be stopped: it runs, or is being restarted. */
export function canStop(runner: Runner): boolean {
  return runner.status === 'running' || runner.status === 'restarting' || runner.status === 'paused'
}

/** The runners of one owner on one forge. */
export interface OwnerGroup {
  key: string
  owner: string
  /** The forge's host: `github.com`. */
  host: string
  runners: Runner[]
}

/**
 * Runners grouped by the user or organization they serve, groups in
 * alphabetical order and runners in the order given. Owners are told apart
 * without case, as forges tell them apart.
 */
export function groupByOwner(runners: Runner[]): OwnerGroup[] {
  const groups = new Map<string, OwnerGroup>()
  for (const runner of runners) {
    const host = new URL(runner.url).host
    const key = `${host}/${runner.owner.toLowerCase()}`
    let group = groups.get(key)
    if (group === undefined) {
      group = { key, owner: runner.owner, host, runners: [] }
      groups.set(key, group)
    }
    group.runners.push(runner)
  }
  return [...groups.values()].sort(
    (a, b) => byText(a.owner, b.owner) || byText(a.host, b.host),
  )
}

/** How many runners there are of each health. */
export function summarize(runners: Runner[]): Record<Health, number> {
  const counts: Record<Health, number> = { up: 0, trouble: 0, down: 0, idle: 0 }
  for (const runner of runners) counts[healthOf(runner)] += 1
  return counts
}

/** Every owner runners serve, each once, in alphabetical order. */
export function ownersOf(runners: Runner[]): string[] {
  return distinct(runners.map((runner) => runner.owner))
}

/** Every repository runners serve (of `owner` alone, if given). */
export function repositoriesOf(runners: Runner[], owner?: string): string[] {
  return distinct(
    runners
      .filter((runner) => !owner || sameName(runner.owner, owner))
      .flatMap((runner) => (runner.repository === null ? [] : [runner.repository])),
  )
}

/** Every label runners are registered with. */
export function labelsOf(runners: Runner[]): string[] {
  return distinct(runners.flatMap((runner) => runner.labels))
}

/** The filters as query parameters, leaving out empty ones. */
export function toQuery(filters: RunnerFilters): URLSearchParams {
  const query = new URLSearchParams()
  for (const [name, value] of Object.entries(filters)) {
    const trimmed = typeof value === 'string' ? value.trim() : ''
    if (trimmed !== '') query.set(name, trimmed)
  }
  return query
}

/** Whether any filter narrows the list. */
export function isFiltering(filters: RunnerFilters): boolean {
  return toQuery(filters).size > 0
}

/** How long ago a runner was created, as a person would say it. */
export function createdAgo(runner: Runner, now: Date = new Date()): string {
  const seconds = Math.round(runner.created_at - now.getTime() / 1000)
  const format = new Intl.RelativeTimeFormat('en', { numeric: 'auto' })
  const units: [Intl.RelativeTimeFormatUnit, number][] = [
    ['day', 86_400],
    ['hour', 3_600],
    ['minute', 60],
  ]
  for (const [unit, length] of units) {
    if (Math.abs(seconds) >= length) return format.format(Math.round(seconds / length), unit)
  }
  return 'just now'
}

function distinct(names: string[]): string[] {
  const kept = new Map<string, string>()
  for (const name of names) {
    if (!kept.has(name.toLowerCase())) kept.set(name.toLowerCase(), name)
  }
  return [...kept.values()].sort(byText)
}

function sameName(a: string, b: string): boolean {
  return a.toLowerCase() === b.toLowerCase()
}

function byText(a: string, b: string): number {
  return a.localeCompare(b, 'en', { sensitivity: 'base' })
}

const memoryUnits: Record<string, string> = { t: 'TiB', g: 'GiB', m: 'MiB', k: 'KiB', b: 'bytes' }

/** A memory limit as a person reads it: `4g` is `4 GiB`. */
export function memoryText(memory: string): string {
  const match = /^(\d+)([tgmkb])$/.exec(memory)
  return match === null ? memory : `${match[1]} ${memoryUnits[match[2]!]}`
}

/** A CPU limit as a person reads it: `1` is `1 CPU`, `0.5` is `0.5 CPUs`. */
export function cpusText(cpus: string): string {
  return cpus === '1' ? '1 CPU' : `${cpus} CPUs`
}

/** What a runner may use of its host, in words; `null` when nothing is limited. */
export function limitsText(runner: Runner): string | null {
  const limits = [
    runner.cpus === null ? null : cpusText(runner.cpus),
    runner.memory === null ? null : `${memoryText(runner.memory)} of memory`,
  ].filter((limit) => limit !== null)
  return limits.length === 0 ? null : `Up to ${limits.join(' and ')}`
}
