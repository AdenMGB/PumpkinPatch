<script setup lang="ts">
withDefaults(
  defineProps<{
    tone?: 'neutral' | 'accent' | 'success' | 'danger' | 'warning'
  }>(),
  { tone: 'neutral' },
)
</script>

<template>
  <span class="pp-badge" :class="`pp-badge--${tone}`"><slot /></span>
</template>

<style scoped>
.pp-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.68rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  padding: 0.2rem 0.5rem;
  border-radius: 999px;
  border: 1px solid transparent;
}
.pp-badge--neutral {
  background: var(--color-surface-hover);
  color: var(--color-text-muted);
}
.pp-badge--accent {
  background: var(--color-accent-muted);
  color: var(--color-accent-hover);
  border-color: rgba(230, 126, 34, 0.35);
}
.pp-badge--success {
  background: var(--color-success-muted);
  color: var(--color-success);
}
.pp-badge--danger {
  background: var(--color-danger-muted);
  color: #fca5a5;
}
.pp-badge--warning {
  background: rgba(234, 179, 8, 0.12);
  color: #facc15;
}
</style>
