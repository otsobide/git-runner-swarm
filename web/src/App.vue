<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'

import CreateRunnerPanel from './components/CreateRunnerPanel.vue'
import FilterBar from './components/FilterBar.vue'
import OwnerRack from './components/OwnerRack.vue'
import RunnerLogs from './components/RunnerLogs.vue'
import SwarmStrip from './components/SwarmStrip.vue'
import type { RunnersGateway } from './core/gateway'
import { groupByOwner, isFiltering, type Runner } from './core/runner'
import { useRunners } from './state/useRunners'

const props = defineProps<{ gateway: RunnersGateway }>()

const {
  shown,
  all,
  images,
  filters,
  loading,
  loadError,
  notice,
  busy,
  create,
  clearFilters,
  start,
  stop,
  remove,
  dismissNotice,
} = useRunners(props.gateway)

const groups = computed(() => groupByOwner(shown.value))
const creating = ref(false)
const announcement = ref<string | null>(null)
const newRunnerButton = ref<HTMLButtonElement | null>(null)

function openPanel() {
  announcement.value = null
  creating.value = true
}

/** Closes the panel, handing focus back to the button that opened it. */
async function closePanel() {
  creating.value = false
  await nextTick()
  newRunnerButton.value?.focus()
}

/** The runner whose logs are open, if any. */
const viewing = ref<Runner | null>(null)

function openLogs(runner: Runner) {
  viewing.value = runner
}

/**
 * Closes the logs, handing focus back to the Logs button of the runner's
 * row, or to New runner if the runner is gone.
 */
async function closeLogs() {
  const id = viewing.value?.id
  viewing.value = null
  await nextTick()
  const button = document.querySelector<HTMLElement>(`[data-logs-of="${id}"]`) ?? newRunnerButton.value
  button?.focus()
}

async function created(runner: Runner) {
  announcement.value = `Created ${runner.name}. It registers with GitHub as its container starts.`
  await closePanel()
}
</script>

<template>
  <div class="page">
    <header class="masthead">
      <h1 class="brand">git-runner-swarm</h1>
      <SwarmStrip class="swarm" :runners="all" />
      <button ref="newRunnerButton" class="button button--primary" type="button" @click="openPanel">
        New runner
      </button>
    </header>

    <div class="messages">
      <p v-if="loadError" class="banner banner--alert" role="alert">
        {{ loadError.message }} The panel keeps trying every few seconds.
      </p>
      <p v-if="notice" class="banner banner--alert" role="alert">
        <span>{{ notice }}</span>
        <button class="button" type="button" @click="dismissNotice">Dismiss</button>
      </p>
      <p v-if="announcement" class="banner" role="status">
        <span>{{ announcement }}</span>
        <button class="button" type="button" @click="announcement = null">Dismiss</button>
      </p>
    </div>

    <FilterBar v-if="all.length > 0" class="filters" :filters="filters" :runners="all" @clear="clearFilters" />

    <main>
      <p v-if="loading" class="quiet">Loading runners…</p>

      <section v-else-if="all.length === 0 && loadError === null" class="empty">
        <h2>No runners yet</h2>
        <p>
          A runner takes jobs from one GitHub repository, or from every repository of an organization. Have the
          registration token GitHub shows under Settings › Actions › Runners › New self-hosted runner at hand, then
          create the runner here.
        </p>
        <button class="button button--primary" type="button" @click="openPanel">New runner</button>
      </section>

      <section v-else-if="shown.length === 0 && isFiltering(filters)" class="empty">
        <h2>No runner matches these filters</h2>
        <button class="button" type="button" @click="clearFilters">Clear filters</button>
      </section>

      <div v-else class="racks">
        <OwnerRack
          v-for="group in groups"
          :key="group.key"
          :group="group"
          :busy="busy"
          @start="start"
          @stop="stop"
          @remove="remove"
          @logs="openLogs"
        />
      </div>
    </main>

    <CreateRunnerPanel v-if="creating" :images="images" :create="create" @created="created" @close="closePanel" />
    <RunnerLogs v-if="viewing" :runner="viewing" :gateway="gateway" @close="closeLogs" />
  </div>
</template>

<style scoped>
.page {
  max-width: 80rem;
  margin: 0 auto;
  padding: var(--space-6) var(--space-4) var(--space-8);
}

.masthead {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-3) var(--space-6);
  margin-bottom: var(--space-6);
}

.brand {
  font-size: var(--text-title);
  font-weight: 700;
  letter-spacing: -0.01em;
  line-height: 1.1;
}

.swarm {
  flex: 1 1 16rem;
}

.messages:empty {
  display: none;
}

.messages {
  display: grid;
  gap: var(--space-2);
  margin-bottom: var(--space-4);
}

.banner {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2) var(--space-4);
  padding: var(--space-3) var(--space-4);
  border: 1px solid var(--rule);
  border-left: 4px solid var(--lamp-up);
  border-radius: var(--radius-control);
  background: var(--rack);
}

.banner--alert {
  border-left-color: var(--lamp-down);
}

.filters {
  margin-bottom: var(--space-6);
}

.racks {
  display: grid;
  gap: var(--space-6);
}

.quiet {
  color: var(--ink-soft);
}

.empty {
  display: grid;
  justify-items: start;
  gap: var(--space-3);
  max-width: 40rem;
  padding: var(--space-8) 0;
}

.empty h2 {
  font-size: var(--text-owner);
  font-weight: 700;
}

.empty p {
  color: var(--ink-soft);
}

@media (min-width: 52rem) {
  .page {
    padding: var(--space-8);
  }
}
</style>
