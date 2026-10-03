<script setup lang="ts">
withDefaults(
  defineProps<{
    label: string
    variant?: 'ghost' | 'danger'
    size?: 'sm' | 'md'
    disabled?: boolean
  }>(),
  { variant: 'ghost', size: 'md', disabled: false },
)
</script>

<template>
  <button
    type="button"
    class="pp-icon-btn"
    :class="[`pp-icon-btn--${variant}`, `pp-icon-btn--${size}`]"
    :aria-label="label"
    :title="label"
    :disabled="disabled"
    data-tauri-drag-region-exclude
  >
    <slot />
  </button>
</template>

<style scoped>
.pp-icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all 0.2s ease;
  color: var(--color-text-muted);
  background: transparent;
}
.pp-icon-btn:hover:not(:disabled) {
  color: var(--color-text);
  background: var(--color-surface-hover);
  transform: scale(1.05);
}
.pp-icon-btn:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--color-accent-ring);
}
.pp-icon-btn--md {
  width: 2.25rem;
  height: 2.25rem;
}
.pp-icon-btn--sm {
  width: 1.85rem;
  height: 1.85rem;
}
.pp-icon-btn--danger:hover:not(:disabled) {
  color: #fca5a5;
  background: var(--color-danger-muted);
}
.pp-icon-btn :deep(svg) {
  width: 1.15rem;
  height: 1.15rem;
}
</style>
