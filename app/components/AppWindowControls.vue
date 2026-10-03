<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { saveWindowState, StateFlags } from '@tauri-apps/plugin-window-state'

const { usesCustomWindowChrome } = useTauriPlatform()

const controlsEl = ref<HTMLElement | null>(null)
const isMaximized = ref(false)

function setWindowControlsWidth(width: number) {
  document.documentElement.style.setProperty('--window-controls-width', `${width}px`)
}

let resizeObserver: ResizeObserver | undefined
let unlistenResize: (() => void) | undefined

watch(controlsEl, (el) => {
  resizeObserver?.disconnect()
  resizeObserver = undefined

  if (!el) {
    setWindowControlsWidth(0)
    return
  }

  resizeObserver = new ResizeObserver(() => {
    setWindowControlsWidth(el.getBoundingClientRect().width)
  })
  resizeObserver.observe(el)
  setWindowControlsWidth(el.getBoundingClientRect().width)
})

watch(
  usesCustomWindowChrome,
  async (show) => {
    unlistenResize?.()
    unlistenResize = undefined
    if (!show) return

    try {
      const window = getCurrentWindow()
      isMaximized.value = await window.isMaximized()
      unlistenResize = await window.onResized(async () => {
        try {
          isMaximized.value = await window.isMaximized()
        } catch {
          /* HMR / reload can invalidate in-flight callbacks */
        }
      })
    } catch {
      /* ignore during dev reload */
    }
  },
  { immediate: true },
)

onUnmounted(() => {
  resizeObserver?.disconnect()
  unlistenResize?.()
  document.documentElement.style.removeProperty('--window-controls-width')
})

async function minimize() {
  try {
    await getCurrentWindow().minimize()
  } catch (error) {
    console.warn('[window] minimize failed', error)
  }
}

async function toggleMaximize() {
  try {
    await getCurrentWindow().toggleMaximize()
    isMaximized.value = await getCurrentWindow().isMaximized()
  } catch (error) {
    console.warn('[window] toggle maximize failed', error)
  }
}

async function closeWindow() {
  try {
    await saveWindowState(StateFlags.ALL)
    await getCurrentWindow().close()
  } catch (error) {
    console.warn('[window] close failed', error)
  }
}
</script>

<template>
  <Teleport to="body">
    <section
      v-if="usesCustomWindowChrome"
      ref="controlsEl"
      class="window-controls"
      data-tauri-drag-region-exclude
      aria-label="Window controls"
    >
      <button type="button" class="control-btn" title="Minimize" @click="minimize">
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <rect x="1" y="5.5" width="10" height="1" fill="currentColor" />
        </svg>
      </button>
      <button type="button" class="control-btn" title="Maximize" @click="toggleMaximize">
        <svg v-if="isMaximized" viewBox="0 0 12 12" aria-hidden="true">
          <path
            fill="currentColor"
            d="M3.5 2h6v6H3.5V2zm1 1v4h4V3h-4zM2 4.5v5.5h5.5V9H3V4.5H2z"
          />
        </svg>
        <svg v-else viewBox="0 0 12 12" aria-hidden="true">
          <rect
            x="2"
            y="2"
            width="8"
            height="8"
            fill="none"
            stroke="currentColor"
            stroke-width="1"
          />
        </svg>
      </button>
      <button
        type="button"
        class="control-btn control-btn--close"
        title="Close"
        @click="closeWindow"
      >
        <svg viewBox="0 0 12 12" aria-hidden="true">
          <path
            fill="currentColor"
            d="M2.2 2.2l7.6 7.6M9.8 2.2L2.2 9.8"
            stroke="currentColor"
            stroke-width="1.2"
            stroke-linecap="round"
          />
        </svg>
      </button>
    </section>
  </Teleport>
</template>

<style scoped>
.window-controls {
  pointer-events: auto;
  position: fixed;
  top: 0;
  right: 0;
  z-index: 10001;
  display: flex;
  align-items: center;
  gap: 0.25rem;
  height: var(--top-bar-height, 3rem);
  padding: 0 0.35rem 0 0.75rem;
  border-bottom-left-radius: 1rem;
  background: var(--color-bg-raised);
  border-left: 1px solid var(--color-border-subtle);
  border-bottom: 1px solid var(--color-border-subtle);
}

.control-btn {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 2.25rem;
  height: 2.25rem;
  border: none;
  border-radius: 0.5rem;
  background: transparent;
  color: var(--color-text);
  cursor: pointer;
  transition:
    background-color 0.2s ease,
    color 0.2s ease,
    transform 0.2s ease;
}

.control-btn::before {
  content: '';
  position: absolute;
  inset: -6px -4px;
}

.control-btn:hover {
  background: var(--color-surface-hover);
  transform: scale(1.02);
}

.control-btn:active {
  transform: scale(0.95);
}

.control-btn:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px var(--color-accent-ring);
}

.control-btn svg {
  width: 0.75rem;
  height: 0.75rem;
}

.control-btn--close:hover {
  background: #e81123;
  color: #fff;
}

.control-btn--close::before {
  inset: -6px -8px -6px -4px;
}
</style>
