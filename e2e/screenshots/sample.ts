/**
 * What the panel is shown in the documentation's screenshots: the
 * runners of two organizations, in every state a runner is seen in, and
 * the images a panel left to its defaults offers. Every owner,
 * repository and runner here is made up.
 */

export interface SampleRunner {
  id: string
  forge: 'github'
  url: string
  owner: string
  repository: string | null
  scope: 'owner' | 'repository'
  name: string
  labels: string[]
  lifecycle: 'persistent' | 'ephemeral'
  cpus: string | null
  memory: string | null
  host_docker: boolean
  host_gpu: boolean
  image: string
  status: 'running' | 'restarting' | 'exited'
  exit_code: number | null
  created_at: number
}

const image = 'ghcr.io/actions/actions-runner:latest'
const now = Math.floor(Date.now() / 1000)
const minutes = 60
const hours = 60 * minutes
const days = 24 * hours

let made = 0

/** A runner of `url`, the rest as a runner is by default unless given. */
function runner(url: string, name: string, rest: Partial<SampleRunner> = {}): SampleRunner {
  made += 1
  const [owner = '', repository = null] = new URL(url).pathname.split('/').filter(Boolean)
  return {
    id: `00000000-0000-4000-8000-${String(made).padStart(12, '0')}`,
    forge: 'github',
    url,
    owner,
    repository,
    scope: repository === null ? 'owner' : 'repository',
    name,
    labels: [],
    lifecycle: 'persistent',
    cpus: null,
    memory: null,
    host_docker: false,
    host_gpu: false,
    image,
    status: 'running',
    exit_code: null,
    created_at: now - days,
    ...rest,
  }
}

const runners: SampleRunner[] = [
  runner('https://github.com/lumen-labs', 'build-1', {
    labels: ['docker', 'large'],
    cpus: '4',
    memory: '8g',
    host_docker: true,
    created_at: now - 12 * days,
  }),
  runner('https://github.com/lumen-labs', 'build-2', {
    labels: ['docker', 'large'],
    cpus: '4',
    memory: '8g',
    host_docker: true,
    created_at: now - 12 * days,
  }),
  runner('https://github.com/lumen-labs/inference', 'gpu-1', {
    labels: ['gpu', 'cuda'],
    memory: '32g',
    host_gpu: true,
    created_at: now - 3 * days,
  }),
  runner('https://github.com/lumen-labs/inference', 'nightly', {
    status: 'exited',
    exit_code: 0,
    created_at: now - 5 * days,
  }),
  runner('https://github.com/lumen-labs/website', 'release', {
    lifecycle: 'ephemeral',
    status: 'exited',
    exit_code: 0,
    created_at: now - 2 * hours,
  }),
  runner('https://github.com/quayside-dev/api', 'api-1', { labels: ['arm64'], cpus: '2', created_at: now - 20 * days }),
  runner('https://github.com/quayside-dev/api', 'api-2', {
    labels: ['arm64'],
    status: 'restarting',
    exit_code: 1,
    created_at: now - 25 * minutes,
  }),
  runner('https://github.com/quayside-dev/docs', 'docs-preview', {
    lifecycle: 'ephemeral',
    status: 'exited',
    exit_code: 1,
    created_at: now - hours,
  }),
]

/** The runners, as the API lists them: newest first, then by name. */
export const sampleRunners: SampleRunner[] = runners.toSorted(
  (a, b) => b.created_at - a.created_at || a.name.localeCompare(b.name),
)

export const sampleImages = [
  {
    key: 'github-official',
    forge: 'github',
    image,
    description: "GitHub's official runner image: the Actions runner on Ubuntu, and nothing else",
  },
]

/** What gpu-1 has logged: registering, connecting, and two jobs. */
export const sampleLog = [
  '',
  '# Authentication',
  '',
  '√ Connected to GitHub',
  '',
  '# Runner Registration',
  '',
  '√ Runner successfully added',
  '',
  '# Runner settings',
  '',
  '√ Settings Saved.',
  '',
  '√ Connected to GitHub',
  '',
  "Current runner version: '2.338.2'",
  '2026-11-07 07:07:20Z: Listening for Jobs',
  '2026-11-07 09:12:41Z: Running job: train (3.12)',
  '2026-11-07 09:31:05Z: Job train (3.12) completed with result: Succeeded',
  '2026-11-08 16:02:17Z: Running job: evaluate',
  '2026-11-08 16:09:52Z: Job evaluate completed with result: Succeeded',
].map((text) => ({ stream: 'stdout', text }))
