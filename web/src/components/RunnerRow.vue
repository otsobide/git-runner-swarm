<script setup lang="ts">
import { computed, ref } from 'vue'

import { canStart, canStop, createdAgo, healthOf, type Runner, statusText } from '../core/runner'
import StatusLamp from './StatusLamp.vue'

const props = defineProps<{ runner: Runner; busy: boolean }>()
const emit = defineEmits<{ start: [Runner]; stop: [Runner]; remove: [Runner] }>()

const health = computed(() => healthOf(props.runner))
/** Deleting asks once more, in the row itself, before it is done. */
const confirming = ref(false)

function remove() {
  confirming.value = false
  emit('remove', props.runner)
}
</script>

<template>
  <li class="row" :class="`row--${health}`">
    <StatusLamp :health="health" />

    <div class="identity">
      <h3 class="name">{{ runner.name }}</h3>
      <p class="status">{{ statusText(runner) }}</p>
    </div>

    <p class="serves">
      <span v-if="runner.repository !== null">{{ runner.repository }}</span>
      <span v-else class="soft">Every repository</span>
      <span v-if="runner.lifecycle === 'ephemeral'" class="soft">, one job at a time</span>
    </p>

    <ul class="labels" aria-label="Labels">
      <li v-for="label in runner.labels" :key="label" class="label mono">{{ label }}</li>
    </ul>

    <p class="created soft">Created {{ createdAgo(runner) }}</p>

    <div class="actions">
      <template v-if="confirming">
        <span class="confirm">Delete {{ runner.name }}?</span>
        <button class="button button--danger" type="button" :disabled="busy" @click="remove">Delete</button>
        <button class="button" type="button" @click="confirming = false">Keep</button>
      </template>
      <template v-else>
        <button v-if="canStop(runner)" class="button" type="button" :disabled="busy" @click="emit('stop', runner)">
          Stop
        </button>
        <button v-if="canStart(runner)" class="button" type="button" :disabled="busy" @click="emit('start', runner)">
          Start
        </button>
        <button class="button button--danger" type="button" :disabled="busy" @click="confirming = true">
          Delete
        </button>
      </template>
    </div>
  </li>
</template>

<style scoped>
.row {
  display: grid;
  grid-template-columns: auto minmax(10rem, 1.4fr) minmax(8rem, 1fr) minmax(6rem, 1fr) auto auto;
  align-items: center;
  gap: var(--space-2) var(--space-4);
  padding: var(--space-3) var(--space-4);
  border-top: 1px solid var(--rule);
}

.row:first-child {
  border-top: none;
}

.name {
  font-size: var(--text-name);
  font-weight: 600;
  line-height: 1.3;
  overflow-wrap: anywhere;
}

.status,
.serves,
.created,
.confirm {
  font-size: var(--text-small);
}

.status {
  color: var(--ink-soft);
}

.row--down .status,
.row--trouble .status {
  color: var(--danger-ink);
  font-weight: 600;
}

.soft {
  color: var(--ink-soft);
}

.labels {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1);
  margin: 0;
  padding: 0;
  list-style: none;
}

.label {
  padding: 0 var(--space-2);
  border: 1px solid var(--rule);
  border-radius: var(--radius-control);
  font-size: var(--text-small);
  line-height: 1.6;
}

.actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-2);
}

@media (max-width: 52rem) {
  .row {
    grid-template-columns: auto 1fr;
  }

  .row > :not(.lamp) {
    grid-column: 2;
  }

  .actions {
    justify-content: flex-start;
  }

  /* Stacked, a runner without labels leaves no gap where they would be. */
  .labels:empty {
    display: none;
  }
}
</style>
