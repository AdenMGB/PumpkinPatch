<script setup lang="ts">
import { downloadProgress } from '~/plugins/patch-init.client'

const pct = computed(() => {
  const p = downloadProgress.value
  if (!p || p.total == null || p.total === 0) return null
  return Math.min(100, Math.round((p.downloaded / p.total) * 100))
})
</script>

<template>
  <div v-if="downloadProgress" class="download-banner" data-tauri-drag-region-exclude>
    <span class="label">{{ downloadProgress.label }}</span>
    <div v-if="pct != null" class="bar-track" aria-hidden="true">
      <div class="bar-fill" :style="{ width: `${pct}%` }" />
    </div>
    <span v-else class="pp-muted">Downloading…</span>
  </div>
</template>

<style scoped>
.download-banner {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.35rem 1rem;
  font-size: 0.8rem;
  border-bottom: 1px solid var(--color-border-subtle);
  background: var(--color-accent-muted);
  color: var(--color-text);
}
.label {
  font-weight: 600;
  white-space: nowrap;
}
.bar-track {
  flex: 1;
  height: 0.35rem;
  border-radius: 999px;
  background: var(--color-border-subtle);
  overflow: hidden;
}
.bar-fill {
  height: 100%;
  background: var(--color-accent);
  transition: width 0.2s ease;
}
</style>
