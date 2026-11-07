<script setup lang="ts">
import type { Health } from '../core/runner'

defineProps<{ health: Health }>()
</script>

<!--
  A runner's lamp, lit in its health's colour. It only repeats what the
  status beside it says in words, so it is hidden from screen readers.
-->
<template>
  <span class="lamp" :class="`lamp--${health}`" aria-hidden="true" />
</template>

<style scoped>
.lamp {
  --lamp: var(--lamp-idle);
  display: inline-block;
  flex: none;
  width: 0.75rem;
  height: 0.75rem;
  border-radius: 50%;
  background: radial-gradient(circle at 35% 35%, color-mix(in srgb, var(--lamp) 55%, white), var(--lamp) 60%);
  box-shadow:
    0 0 0 1px color-mix(in srgb, var(--lamp) 70%, black),
    0 0 0.5rem color-mix(in srgb, var(--lamp) 60%, transparent);
}

.lamp--up {
  --lamp: var(--lamp-up);
}

.lamp--down {
  --lamp: var(--lamp-down);
}

/* Being restarted over and over: the one lamp that moves. */
.lamp--trouble {
  --lamp: var(--lamp-trouble);
  animation: blink 1.2s steps(2, jump-none) infinite;
}

/* Stopped as meant: unlit. */
.lamp--idle {
  background: var(--lamp-idle);
  box-shadow: inset 0 1px 2px color-mix(in srgb, black 30%, transparent);
}

@keyframes blink {
  50% {
    opacity: 0.35;
  }
}
</style>
