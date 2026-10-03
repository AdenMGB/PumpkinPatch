<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger' | 'quiet'
    size?: 'sm' | 'md' | 'lg'
    type?: 'button' | 'submit'
    disabled?: boolean
    loading?: boolean
  }>(),
  { variant: 'primary', size: 'md', type: 'button', disabled: false, loading: false },
)
</script>

<template>
  <button
    :type="type"
    class="pp-btn"
    :class="[`pp-btn--${variant}`, `pp-btn--${size}`]"
    :disabled="disabled || loading"
    data-tauri-drag-region-exclude
  >
    <span v-if="loading" class="pp-btn__spinner" aria-hidden="true" />
    <slot />
  </button>
</template>

<style scoped>
.pp-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.4rem;
  font-weight: 600;
  border-radius: var(--radius-md);
  border: 1px solid transparent;
  cursor: pointer;
  transition: all 0.2s ease;
  white-space: nowrap;
}
.pp-btn:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--color-bg), 0 0 0 4px var(--color-accent-ring);
}
.pp-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none !important;
}
.pp-btn--md {
  padding: 0.5rem 1rem;
  font-size: 0.875rem;
}
.pp-btn--sm {
  padding: 0.35rem 0.65rem;
  font-size: 0.8rem;
}
.pp-btn--lg {
  padding: 0.65rem 1.25rem;
  font-size: 0.95rem;
}
.pp-btn--primary {
  background: var(--color-accent);
  color: #1a0f05;
}
.pp-btn--primary:hover:not(:disabled) {
  background: var(--color-accent-hover);
  transform: scale(1.05);
}
.pp-btn--primary:active:not(:disabled) {
  transform: scale(0.95);
}
.pp-btn--secondary {
  background: var(--color-surface);
  border-color: var(--color-border-subtle);
  color: var(--color-text);
}
.pp-btn--secondary:hover:not(:disabled) {
  background: var(--color-surface-hover);
  transform: scale(1.02);
}
.pp-btn--ghost,
.pp-btn--quiet {
  background: transparent;
  color: var(--color-text-muted);
}
.pp-btn--ghost:hover:not(:disabled),
.pp-btn--quiet:hover:not(:disabled) {
  color: var(--color-text);
  background: var(--color-surface-hover);
}
.pp-btn--danger {
  background: var(--color-danger-muted);
  color: #fca5a5;
  border-color: rgba(239, 68, 68, 0.35);
}
.pp-btn--danger:hover:not(:disabled) {
  background: rgba(239, 68, 68, 0.22);
  transform: scale(1.02);
}
.pp-btn__spinner {
  width: 0.9rem;
  height: 0.9rem;
  border: 2px solid currentColor;
  border-right-color: transparent;
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
