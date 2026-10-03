<script setup lang="ts">
import type { NetworkHealthReport } from '~/types/patch'

defineProps<{
  report: NetworkHealthReport | undefined
  loading?: boolean
}>()

function tone(level: string) {
  if (level === 'healthy') return 'success'
  if (level === 'degraded') return 'warning'
  if (level === 'down') return 'danger'
  return 'neutral'
}
</script>

<template>
  <PpCard padding="md" class="health">
    <div class="row">
      <h3>Health</h3>
      <PpBadge v-if="report" :tone="tone(report.overall)">{{ report.overall }}</PpBadge>
      <span v-else-if="loading" class="pp-muted">Checking…</span>
    </div>
    <ul v-if="report" class="list">
      <li>
        <PpBadge :tone="tone(report.hub.level)">hub</PpBadge>
        {{ report.hub.detail }}
      </li>
      <li v-for="s in report.servers" :key="s.id">
        <PpBadge :tone="tone(s.level)">{{ s.label }}</PpBadge>
        {{ s.detail }}
      </li>
      <li v-for="p in report.plugins" :key="p.id">
        <PpBadge :tone="tone(p.level)">plugin</PpBadge>
        {{ p.label }} — {{ p.detail }}
      </li>
    </ul>
  </PpCard>
</template>

<style scoped>
.health h3 {
  margin: 0;
  font-size: 0.95rem;
}
.row {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  margin-bottom: 0.65rem;
}
.list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  font-size: 0.85rem;
}
.list li {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
  align-items: baseline;
}
</style>
