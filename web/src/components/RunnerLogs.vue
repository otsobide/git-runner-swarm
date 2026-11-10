<script setup lang="ts">
import { nextTick, onMounted, ref, useId, watch } from 'vue'

import type { RunnersGateway } from '../core/gateway'
import { LOG_LINES, type Runner } from '../core/runner'
import { useRunnerLogs } from '../state/useRunnerLogs'

const props = defineProps<{ runner: Runner; gateway: RunnersGateway }>()
const emit = defineEmits<{ close: [] }>()

const { lines, loading, error } = useRunnerLogs(props.gateway, props.runner.id)
const id = useId()
const log = ref<HTMLElement | null>(null)
const closeButton = ref<HTMLButtonElement | null>(null)

onMounted(() => closeButton.value?.focus())

/**
 * Keeps the latest line in view as lines come in, unless the log was
 * scrolled up to read something, which is then left where it is.
 */
watch(lines, async () => {
  const element = log.value
  const following = element === null || element.scrollHeight - element.scrollTop - element.clientHeight < 32
  await nextTick()
  if (following && log.value !== null) log.value.scrollTop = log.value.scrollHeight
})
</script>

<template>
  <div class="backdrop">
    <section
      class="panel"
      role="dialog"
      aria-modal="true"
      :aria-labelledby="`${id}-title`"
      :aria-describedby="`${id}-about`"
      @keydown.esc="emit('close')"
    >
      <header class="head">
        <div>
          <h2 :id="`${id}-title`" class="title">Logs of {{ runner.name }}</h2>
          <p :id="`${id}-about`" class="about">
            Its last {{ LOG_LINES }} lines: the runner registering and connecting, then the jobs it runs. Kept
            current every few seconds.
          </p>
        </div>
        <button ref="closeButton" class="button" type="button" @click="emit('close')">Close</button>
      </header>

      <p v-if="error" class="failure" role="alert">
        {{ error.message }} The panel keeps trying every few seconds.
      </p>

      <p v-if="loading" class="quiet">Reading the log…</p>
      <p v-else-if="lines.length === 0 && error === null" class="quiet">It has logged nothing yet.</p>
      <pre v-else-if="lines.length > 0" ref="log" class="log mono" tabindex="0" :aria-label="`Log of ${runner.name}`"><span v-for="(line, index) in lines" :key="index" :class="['line', `line--${line.stream}`]">{{ line.text }}{{ '\n' }}</span></pre>
    </section>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  display: flex;
  justify-content: flex-end;
  background: var(--scrim);
  z-index: 10;
}

.panel {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  width: min(56rem, 100%);
  height: 100%;
  padding: var(--space-6);
  background: var(--rack);
  border-left: 1px solid var(--rule);
  animation: slide-in 180ms ease-out;
}

.head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-4);
}

.title {
  font-size: var(--text-owner);
  font-weight: 700;
  overflow-wrap: anywhere;
}

.about,
.quiet {
  max-width: 40rem;
  color: var(--ink-soft);
  font-size: var(--text-small);
}

.failure {
  color: var(--danger-ink);
  font-size: var(--text-small);
  font-weight: 600;
}

.log {
  flex: 1;
  min-height: 0;
  margin: 0;
  padding: var(--space-3) var(--space-4);
  overflow: auto;
  border: 1px solid var(--rule);
  border-radius: var(--radius-control);
  background: var(--field);
  font-size: var(--text-small);
  line-height: 1.55;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

@keyframes slide-in {
  from {
    transform: translateX(2rem);
    opacity: 0;
  }
}
</style>
