<script setup lang="ts">
import { computed, useId } from 'vue'

import {
  isFiltering,
  labelsOf,
  ownersOf,
  repositoriesOf,
  type Runner,
  type RunnerFilters,
  type RunnerStatus,
} from '../core/runner'

/**
 * `filters` is the panel's own, changed in place as the fields change;
 * `runners` is every runner, which the options are drawn from.
 */
const props = defineProps<{ filters: Required<RunnerFilters>; runners: Runner[] }>()
const emit = defineEmits<{ clear: [] }>()

const statuses: [RunnerStatus, string][] = [
  ['running', 'Running'],
  ['restarting', 'Restarting'],
  ['exited', 'Stopped or failed'],
  ['created', 'Created'],
  ['paused', 'Paused'],
  ['dead', 'Dead'],
]

const owners = computed(() => ownersOf(props.runners))
const repositories = computed(() => repositoriesOf(props.runners, props.filters.owner))
const labels = computed(() => labelsOf(props.runners))
/** Each field is named by its own label alone, not the option it shows. */
const id = useId()
</script>

<template>
  <form class="filters" role="search" aria-label="Filter runners" @submit.prevent>
    <div class="field search">
      <label :for="`${id}-q`" class="visually-hidden">Search runners</label>
      <input
        :id="`${id}-q`"
        v-model="filters.q"
        type="search"
        placeholder="Search names and URLs"
        autocomplete="off"
      />
    </div>

    <div class="field">
      <label :for="`${id}-owner`" class="visually-hidden">Owner</label>
      <select :id="`${id}-owner`" v-model="filters.owner">
        <option value="">Every owner</option>
        <option v-for="owner in owners" :key="owner" :value="owner">{{ owner }}</option>
      </select>
    </div>

    <div class="field">
      <label :for="`${id}-repository`" class="visually-hidden">Repository</label>
      <select :id="`${id}-repository`" v-model="filters.repository" :disabled="repositories.length === 0">
        <option value="">Every repository</option>
        <option v-for="repository in repositories" :key="repository" :value="repository">{{ repository }}</option>
      </select>
    </div>

    <div class="field">
      <label :for="`${id}-status`" class="visually-hidden">Status</label>
      <select :id="`${id}-status`" v-model="filters.status">
        <option value="">Any status</option>
        <option v-for="[status, text] in statuses" :key="status" :value="status">{{ text }}</option>
      </select>
    </div>

    <div v-if="labels.length > 0" class="field">
      <label :for="`${id}-label`" class="visually-hidden">Label</label>
      <select :id="`${id}-label`" v-model="filters.label">
        <option value="">Any label</option>
        <option v-for="label in labels" :key="label" :value="label">{{ label }}</option>
      </select>
    </div>

    <button v-if="isFiltering(filters)" class="button" type="button" @click="emit('clear')">Clear filters</button>
  </form>
</template>

<style scoped>
.filters {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.field {
  flex: 1 1 9rem;
}

.field.search {
  flex: 3 1 14rem;
}

input,
select {
  width: 100%;
  min-height: 2.25rem;
  padding: 0 var(--space-3);
  border: 1px solid var(--rule);
  border-radius: var(--radius-control);
  background: var(--field);
  font-size: var(--text-small);
}

select {
  padding-right: var(--space-6);
}

select:disabled {
  opacity: 0.55;
}
</style>
