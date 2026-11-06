<script setup lang="ts">
import { computed } from 'vue'

import { type Health, healthOf, type Runner, statusText, summarize } from '../core/runner'
import StatusLamp from './StatusLamp.vue'

const props = defineProps<{ runners: Runner[] }>()

/** Worst first, so what needs looking at is where the eye starts. */
const order: Health[] = ['down', 'trouble', 'up', 'idle']
const words: Record<Health, string> = { down: 'failed', trouble: 'restarting', up: 'running', idle: 'stopped' }

const lamps = computed(() =>
  props.runners
    .map((runner) => ({ runner, health: healthOf(runner) }))
    .sort((a, b) => order.indexOf(a.health) - order.indexOf(b.health)),
)

const counts = computed(() => {
  const counts = summarize(props.runners)
  return order.filter((health) => counts[health] > 0).map((health) => `${counts[health]} ${words[health]}`)
})
</script>

<!--
  Every runner's lamp in a row, as on the front of a rack: the swarm's
  health at a glance, before any rack is read. The counts say it in words.
-->
<template>
  <div class="strip">
    <ul class="lamps" aria-hidden="true">
      <li v-for="{ runner, health } in lamps" :key="runner.id" :title="`${runner.name}: ${statusText(runner)}`">
        <StatusLamp :health="health" />
      </li>
    </ul>
    <p class="counts">{{ counts.length > 0 ? counts.join(', ') : 'No runners' }}</p>
  </div>
</template>

<style scoped>
.strip {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2) var(--space-4);
  min-width: 0;
}

.lamps {
  display: flex;
  flex-wrap: wrap;
  gap: 0.375rem;
  margin: 0;
  padding: 0.375rem 0.5rem;
  list-style: none;
  border: 1px solid var(--rule);
  border-radius: var(--radius-control);
  background: var(--rack-header);
}

.lamps:empty {
  display: none;
}

.lamps li {
  display: flex;
}

.counts {
  color: var(--ink-soft);
  font-size: var(--text-small);
}
</style>
