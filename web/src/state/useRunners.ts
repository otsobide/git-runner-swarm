import { onScopeDispose, reactive, ref, watch } from 'vue'

import { asGatewayError, type GatewayError, type RunnersGateway } from '../core/gateway'
import {
  isFiltering,
  type NewRunner,
  repositoriesOf,
  type Runner,
  type RunnerFilters,
  type RunnerImage,
} from '../core/runner'

/** How often the panel asks for runners again, so their states stay current. */
export const POLL_EVERY_MS = 5_000

function noFilters(): Required<RunnerFilters> {
  return { owner: '', repository: '', scope: '', status: '', label: '', q: '' }
}

/**
 * The panel's state: the runners the filters match, every runner (for the
 * summary and the filters' options), the images runners can be created
 * from, and what is under way. Kept current by asking again every
 * `pollEvery` milliseconds, until the scope it was created in ends.
 */
export function useRunners(gateway: RunnersGateway, pollEvery = POLL_EVERY_MS) {
  const shown = ref<Runner[]>([])
  const all = ref<Runner[]>([])
  const images = ref<RunnerImage[]>([])
  const filters = reactive(noFilters())
  const loading = ref(true)
  /** Why the runners could not be loaded the last time they were asked for. */
  const loadError = ref<GatewayError | null>(null)
  /** Why the last thing done to a runner failed, until it is dismissed. */
  const notice = ref<string | null>(null)
  /** The runners something is being done to. */
  const busy = reactive(new Set<string>())

  // Each refresh is numbered, and only the latest one's answer is kept:
  // filters changed while a list was on its way make that list stale.
  let latest = 0

  async function refresh(): Promise<void> {
    const mine = ++latest
    const filtering = isFiltering(filters)
    try {
      const [every, matching] = await Promise.all([
        gateway.listRunners(),
        filtering ? gateway.listRunners({ ...filters }) : Promise.resolve(null),
      ])
      if (mine !== latest) return
      all.value = every
      shown.value = matching ?? every
      loadError.value = null
    } catch (error) {
      if (mine === latest) loadError.value = asGatewayError(error)
    } finally {
      if (mine === latest) loading.value = false
    }
  }

  async function loadImages(): Promise<void> {
    try {
      images.value = await gateway.listImages()
    } catch (error) {
      loadError.value = asGatewayError(error)
    }
  }

  async function act(runner: Runner, action: (id: string) => Promise<unknown>): Promise<void> {
    busy.add(runner.id)
    try {
      await action(runner.id)
    } catch (error) {
      notice.value = asGatewayError(error).message
    } finally {
      busy.delete(runner.id)
    }
    await refresh()
  }

  /** Creates a runner. Fails as the gateway does, for the form to show why. */
  async function create(newRunner: NewRunner): Promise<Runner> {
    const runner = await gateway.createRunner(newRunner)
    await refresh()
    return runner
  }

  function clearFilters(): void {
    Object.assign(filters, noFilters())
  }

  // A repository chosen for an owner no longer chosen would match nothing.
  watch(
    () => filters.owner,
    (owner) => {
      if (filters.repository && !repositoriesOf(all.value, owner).includes(filters.repository)) {
        filters.repository = ''
      }
    },
  )
  watch(filters, () => void refresh(), { deep: true })

  void refresh()
  void loadImages()
  const poll = setInterval(() => void refresh(), pollEvery)
  onScopeDispose(() => clearInterval(poll))

  return {
    shown,
    all,
    images,
    filters,
    loading,
    loadError,
    notice,
    busy,
    refresh,
    create,
    clearFilters,
    start: (runner: Runner) => act(runner, (id) => gateway.startRunner(id)),
    stop: (runner: Runner) => act(runner, (id) => gateway.stopRunner(id)),
    remove: (runner: Runner) => act(runner, (id) => gateway.deleteRunner(id)),
    dismissNotice: () => {
      notice.value = null
    },
  }
}

export type RunnersState = ReturnType<typeof useRunners>
