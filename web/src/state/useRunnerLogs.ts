import { onScopeDispose, ref } from 'vue'

import { asGatewayError, type GatewayError, type RunnersGateway } from '../core/gateway'
import type { LogLine } from '../core/runner'

/** How long after each answer an open log is asked for again. */
export const LOG_POLL_EVERY_MS = 3_000

/**
 * A runner's latest log lines, kept current for as long as the scope it
 * was created in lasts. The log is asked for again `pollEvery`
 * milliseconds after each answer, not on a fixed beat, so a slow answer
 * never has another queued behind it.
 */
export function useRunnerLogs(gateway: RunnersGateway, id: string, pollEvery = LOG_POLL_EVERY_MS) {
  const lines = ref<LogLine[]>([])
  const loading = ref(true)
  /** Why the log could not be read the last time it was asked for. */
  const error = ref<GatewayError | null>(null)

  let ended = false
  let next: ReturnType<typeof setTimeout> | undefined

  async function refresh(): Promise<void> {
    try {
      lines.value = await gateway.runnerLogs(id)
      error.value = null
    } catch (failure) {
      error.value = asGatewayError(failure)
    } finally {
      loading.value = false
    }
    if (!ended) next = setTimeout(() => void refresh(), pollEvery)
  }

  void refresh()
  onScopeDispose(() => {
    ended = true
    clearTimeout(next)
  })

  return { lines, loading, error }
}
