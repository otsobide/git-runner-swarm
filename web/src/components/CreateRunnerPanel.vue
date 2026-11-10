<script setup lang="ts">
import { computed, onMounted, reactive, ref, useId } from 'vue'

import { GatewayError } from '../core/gateway'
import type { Lifecycle, NewRunner, Runner, RunnerImage } from '../core/runner'

const props = defineProps<{
  images: RunnerImage[]
  /** Creates the runner; fails as the gateway does when it cannot. */
  create: (runner: NewRunner) => Promise<Runner>
}>()
const emit = defineEmits<{ created: [Runner]; close: [] }>()

type Field = 'url' | 'token' | 'name' | 'labels' | 'lifecycle' | 'cpus' | 'memory' | 'image'

/** The fields the API names, as it names them, and the inputs they are. */
const fieldNamed: Record<string, Field> = {
  URL: 'url',
  'registration token': 'token',
  'runner name': 'name',
  label: 'labels',
  lifecycle: 'lifecycle',
  'CPU limit': 'cpus',
  'memory limit': 'memory',
  image: 'image',
}

const form = reactive({
  url: '',
  token: '',
  name: '',
  labels: '',
  lifecycle: 'persistent' as Lifecycle,
  cpus: '',
  memory: '',
  hostDocker: false,
  image: props.images[0]?.key ?? '',
})
const fieldErrors = reactive<Partial<Record<Field, string>>>({})
const formError = ref<string | null>(null)
const pending = ref(false)
const firstField = ref<HTMLInputElement | null>(null)
const id = useId()

const chosenImage = computed(() => props.images.find((image) => image.key === form.image))

onMounted(() => firstField.value?.focus())

async function submit() {
  pending.value = true
  formError.value = null
  for (const field of Object.keys(fieldErrors) as Field[]) delete fieldErrors[field]
  try {
    const asked: NewRunner = {
      url: form.url,
      token: form.token,
      name: form.name,
      labels: form.labels.split(/[\s,]+/).filter((label) => label !== ''),
      lifecycle: form.lifecycle,
      image: form.image,
    }
    // A limit left blank is no limit.
    if (form.cpus.trim() !== '') asked.cpus = form.cpus.trim()
    if (form.memory.trim() !== '') asked.memory = form.memory.trim()
    if (form.hostDocker) asked.host_docker = true
    const runner = await props.create(asked)
    emit('created', runner)
  } catch (error) {
    const field = error instanceof GatewayError && error.field !== null ? fieldNamed[error.field] : undefined
    const message = error instanceof Error ? error.message : String(error)
    if (field === undefined) formError.value = sentence(message)
    else fieldErrors[field] = sentence(message.replace(/^invalid [^:]+: /, ''))
  } finally {
    pending.value = false
  }
}

/** A message as a sentence: capitalized, and ending with a full stop. */
function sentence(message: string): string {
  const text = message.charAt(0).toUpperCase() + message.slice(1)
  return /[.?!]$/.test(text) ? text : `${text}.`
}
</script>

<template>
  <div class="backdrop">
    <section
      class="panel"
      role="dialog"
      aria-modal="true"
      :aria-labelledby="`${id}-title`"
      @keydown.esc="emit('close')"
    >
      <h2 :id="`${id}-title`" class="title">New runner</h2>

      <form novalidate @submit.prevent="submit">
        <p v-if="formError" class="form-error" role="alert">{{ formError }}</p>

        <div class="field">
          <label :for="`${id}-url`">Repository or owner URL</label>
          <input
            :id="`${id}-url`"
            ref="firstField"
            v-model="form.url"
            type="url"
            required
            placeholder="https://github.com/owner/repository"
            :aria-invalid="fieldErrors.url !== undefined"
            :aria-describedby="`${id}-url-hint`"
          />
          <p v-if="fieldErrors.url" class="error" role="alert">{{ fieldErrors.url }}</p>
          <p :id="`${id}-url-hint`" class="hint">
            A repository’s URL for a runner of that repository; an organization’s for a runner of every repository
            it owns. GitHub shares no runner across a personal account’s repositories.
          </p>
        </div>

        <div class="field">
          <label :for="`${id}-token`">Registration token</label>
          <input
            :id="`${id}-token`"
            v-model="form.token"
            type="password"
            required
            autocomplete="off"
            spellcheck="false"
            :aria-invalid="fieldErrors.token !== undefined"
            :aria-describedby="`${id}-token-hint`"
          />
          <p v-if="fieldErrors.token" class="error" role="alert">{{ fieldErrors.token }}</p>
          <p :id="`${id}-token-hint`" class="hint">
            On GitHub, under Settings › Actions › Runners › New self-hosted runner, for the repository or the
            organization. It expires an hour after GitHub shows it.
          </p>
        </div>

        <div class="field">
          <label :for="`${id}-name`">Name</label>
          <input
            :id="`${id}-name`"
            v-model="form.name"
            type="text"
            required
            autocomplete="off"
            spellcheck="false"
            placeholder="builder-1"
            :aria-invalid="fieldErrors.name !== undefined"
            :aria-describedby="`${id}-name-hint`"
          />
          <p v-if="fieldErrors.name" class="error" role="alert">{{ fieldErrors.name }}</p>
          <p :id="`${id}-name-hint`" class="hint">What GitHub lists it as: letters, digits, -, _ and .</p>
        </div>

        <div class="field">
          <label :for="`${id}-labels`">Labels</label>
          <input
            :id="`${id}-labels`"
            v-model="form.labels"
            type="text"
            autocomplete="off"
            spellcheck="false"
            placeholder="gpu, large"
            :aria-invalid="fieldErrors.labels !== undefined"
            :aria-describedby="`${id}-labels-hint`"
          />
          <p v-if="fieldErrors.labels" class="error" role="alert">{{ fieldErrors.labels }}</p>
          <p :id="`${id}-labels-hint`" class="hint">
            Extra labels to pick it by in <code>runs-on</code>, separated by commas. GitHub gives every runner
            <code>self-hosted</code>, its OS and its architecture on its own.
          </p>
        </div>

        <fieldset class="field">
          <legend>Jobs</legend>
          <label class="choice">
            <input v-model="form.lifecycle" type="radio" name="lifecycle" value="persistent" />
            <span>
              <strong>Job after job</strong>
              <span class="hint">Keeps taking jobs until it is stopped.</span>
            </span>
          </label>
          <label class="choice">
            <input v-model="form.lifecycle" type="radio" name="lifecycle" value="ephemeral" />
            <span>
              <strong>One job</strong>
              <span class="hint">Takes a single job and stops, so no job ever runs where another ran.</span>
            </span>
          </label>
          <p v-if="fieldErrors.lifecycle" class="error" role="alert">{{ fieldErrors.lifecycle }}</p>
        </fieldset>

        <fieldset class="field" :aria-describedby="`${id}-limits-hint`">
          <legend>Limits</legend>
          <div class="pair">
            <div class="part">
              <label :for="`${id}-cpus`">CPUs</label>
              <input
                :id="`${id}-cpus`"
                v-model="form.cpus"
                type="text"
                inputmode="decimal"
                autocomplete="off"
                spellcheck="false"
                placeholder="No limit"
                :aria-invalid="fieldErrors.cpus !== undefined"
                :aria-describedby="`${id}-cpus-hint`"
              />
              <p :id="`${id}-cpus-hint`" class="hint">Such as 2 or 0.5.</p>
            </div>
            <div class="part">
              <label :for="`${id}-memory`">Memory</label>
              <input
                :id="`${id}-memory`"
                v-model="form.memory"
                type="text"
                autocomplete="off"
                spellcheck="false"
                placeholder="No limit"
                :aria-invalid="fieldErrors.memory !== undefined"
                :aria-describedby="`${id}-memory-hint`"
              />
              <p :id="`${id}-memory-hint`" class="hint">Such as 4g or 512m.</p>
            </div>
          </div>
          <p v-if="fieldErrors.cpus" class="error" role="alert">{{ fieldErrors.cpus }}</p>
          <p v-if="fieldErrors.memory" class="error" role="alert">{{ fieldErrors.memory }}</p>
          <p :id="`${id}-limits-hint`" class="hint">
            What the runner and its jobs may use of this host. Left empty, they may use all of it.
          </p>
        </fieldset>

        <fieldset class="field">
          <legend>Docker</legend>
          <label class="choice">
            <input v-model="form.hostDocker" type="checkbox" :aria-describedby="`${id}-docker-hint`" />
            <span>
              <strong>Give its jobs this host’s Docker</strong>
              <span :id="`${id}-docker-hint`" class="hint">
                So they can build images and run containers. They then control this host as root does: only for
                repositories whose every workflow you trust.
              </span>
            </span>
          </label>
        </fieldset>

        <div v-if="images.length > 1" class="field">
          <label :for="`${id}-image`">Image</label>
          <select :id="`${id}-image`" v-model="form.image" :aria-invalid="fieldErrors.image !== undefined">
            <option v-for="image in images" :key="image.key" :value="image.key">{{ image.description }}</option>
          </select>
          <p v-if="fieldErrors.image" class="error" role="alert">{{ fieldErrors.image }}</p>
        </div>
        <p v-if="chosenImage" class="hint">
          Runs <code>{{ chosenImage.image }}</code>, pulled first if Docker does not have it.
        </p>
        <p v-if="images.length <= 1 && fieldErrors.image" class="error" role="alert">{{ fieldErrors.image }}</p>

        <div class="buttons">
          <button class="button button--primary" type="submit" :disabled="pending">
            {{ pending ? 'Creating runner…' : 'Create runner' }}
          </button>
          <button class="button" type="button" @click="emit('close')">Cancel</button>
        </div>
      </form>
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
  width: min(32rem, 100%);
  height: 100%;
  overflow-y: auto;
  padding: var(--space-6);
  background: var(--rack);
  border-left: 1px solid var(--rule);
  animation: slide-in 180ms ease-out;
}

.title {
  margin-bottom: var(--space-6);
  font-size: var(--text-owner);
  font-weight: 700;
}

.field {
  display: grid;
  gap: var(--space-1);
  margin: 0 0 var(--space-6);
  padding: 0;
  border: none;
}

label,
legend {
  font-weight: 600;
}

legend {
  margin-bottom: var(--space-2);
  padding: 0;
}

input[type='url'],
input[type='password'],
input[type='text'],
select {
  min-height: 2.5rem;
  padding: 0 var(--space-3);
  border: 1px solid var(--rule);
  border-radius: var(--radius-control);
  background: var(--field);
}

input[aria-invalid='true'],
select[aria-invalid='true'] {
  border-color: var(--danger-ink);
}

.hint {
  color: var(--ink-soft);
  font-size: var(--text-small);
  font-weight: 400;
  max-width: 34rem;
}

.error,
.form-error {
  color: var(--danger-ink);
  font-size: var(--text-small);
  font-weight: 600;
}

.form-error {
  margin-bottom: var(--space-4);
}

.choice {
  display: flex;
  gap: var(--space-3);
  align-items: baseline;
  margin-bottom: var(--space-2);
  font-weight: 400;
}

.choice > span {
  display: grid;
}

.pair {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-3);
}

.part {
  display: grid;
  gap: var(--space-1);
  align-content: start;
}

.buttons {
  display: flex;
  gap: var(--space-2);
  margin-top: var(--space-6);
}

@keyframes slide-in {
  from {
    transform: translateX(2rem);
    opacity: 0;
  }
}
</style>
