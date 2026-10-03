<script setup lang="ts">
import { XMarkIcon } from '@heroicons/vue/24/outline'

const props = withDefaults(
  defineProps<{
    open: boolean
    title: string
    confirmLabel?: string
    cancelLabel?: string
    danger?: boolean
    loading?: boolean
  }>(),
  {
    confirmLabel: 'Confirm',
    cancelLabel: 'Cancel',
    danger: false,
    loading: false,
  },
)

const emit = defineEmits<{
  close: []
  confirm: []
}>()
</script>

<template>
  <Teleport to="body">
    <Transition name="pp-modal">
      <div
        v-if="open"
        class="backdrop"
        role="presentation"
        data-tauri-drag-region-exclude
        @click.self="emit('close')"
      >
        <div
          class="dialog"
          role="dialog"
          aria-modal="true"
          :aria-labelledby="title"
        >
          <header class="head">
            <h2 :id="title">{{ title }}</h2>
            <button type="button" class="close" aria-label="Close" @click="emit('close')">
              <XMarkIcon class="ico" />
            </button>
          </header>
          <div class="body">
            <slot />
          </div>
          <footer class="foot">
            <PpButton variant="secondary" @click="emit('close')">{{ cancelLabel }}</PpButton>
            <PpButton
              :variant="danger ? 'danger' : 'primary'"
              :loading="loading"
              @click="emit('confirm')"
            >
              {{ confirmLabel }}
            </PpButton>
          </footer>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1rem;
  background: rgb(0 0 0 / 50%);
  backdrop-filter: blur(4px);
}
.dialog {
  width: min(100%, 28rem);
  background: var(--color-bg-elevated);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  overflow: hidden;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem 1.15rem;
  border-bottom: 1px solid var(--color-border-subtle);
}
.head h2 {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 600;
}
.close {
  border: none;
  background: transparent;
  color: var(--color-text-muted);
  padding: 0.35rem;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all 0.2s ease-in-out;
}
.close:hover {
  color: var(--color-text);
  transform: scale(1.05);
}
.close .ico {
  width: 1.25rem;
  height: 1.25rem;
}
.body {
  padding: 1rem 1.15rem;
  color: var(--color-text);
  line-height: 1.5;
}
.foot {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  padding: 0.85rem 1.15rem 1rem;
  border-top: 1px solid var(--color-border-subtle);
}
.pp-modal-enter-active,
.pp-modal-leave-active {
  transition: opacity 0.2s ease-in-out;
}
.pp-modal-enter-from,
.pp-modal-leave-to {
  opacity: 0;
}
</style>
