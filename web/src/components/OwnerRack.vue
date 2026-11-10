<script setup lang="ts">
import { computed } from 'vue'

import type { OwnerGroup, Runner } from '../core/runner'
import RunnerRow from './RunnerRow.vue'

const props = defineProps<{ group: OwnerGroup; busy: ReadonlySet<string> }>()
const emit = defineEmits<{ start: [Runner]; stop: [Runner]; remove: [Runner]; logs: [Runner] }>()

const count = computed(() => {
  const runners = props.group.runners.length
  return runners === 1 ? '1 runner' : `${runners} runners`
})
/** The forge is named only when it is not GitHub itself, where most are. */
const onOtherForge = computed(() => props.group.host !== 'github.com')
</script>

<!-- The runners of one user or organization, together, as one rack. -->
<template>
  <section class="rack" :aria-labelledby="`rack-${group.key}`">
    <header class="header">
      <h2 :id="`rack-${group.key}`" class="owner">{{ group.owner }}</h2>
      <p v-if="onOtherForge" class="host">on {{ group.host }}</p>
      <p class="count">{{ count }}</p>
    </header>
    <ul class="runners">
      <RunnerRow
        v-for="runner in group.runners"
        :key="runner.id"
        :runner="runner"
        :busy="busy.has(runner.id)"
        @start="emit('start', $event)"
        @stop="emit('stop', $event)"
        @remove="emit('remove', $event)"
        @logs="emit('logs', $event)"
      />
    </ul>
  </section>
</template>

<style scoped>
.rack {
  border: 1px solid var(--rule);
  border-radius: var(--radius-rack);
  background: var(--rack);
  overflow: hidden;
}

.header {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--space-1) var(--space-3);
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--rule);
  background: var(--rack-header);
}

.owner {
  font-size: var(--text-owner);
  font-weight: 700;
  line-height: 1.2;
  overflow-wrap: anywhere;
}

.host,
.count {
  color: var(--ink-soft);
  font-size: var(--text-small);
}

.count {
  margin-left: auto;
}

.runners {
  margin: 0;
  padding: 0;
  list-style: none;
}
</style>
