<script setup lang="ts">
import { useQuery } from '@tanstack/vue-query'
import type { NetworkAnalyticsOverview } from '~/types/patch'

const route = useRoute()
const api = usePatchApi()
const id = computed(() => route.params.id as string)

const selectedServerId = ref<string | 'combined'>('combined')

const { data: overview, isLoading, refetch, isFetching } = useQuery({
  queryKey: ['analytics', id],
  queryFn: () => api.analyticsNetworkOverview(id.value),
  refetchInterval: 30_000,
})

const selectedSnapshot = computed(() => {
  if (!overview.value) return null
  if (selectedServerId.value === 'combined') {
    return null
  }
  return overview.value.servers.find((s) => s.server_id === selectedServerId.value) ?? null
})

function formatDuration(secs: number) {
  if (secs < 60) return `${secs}s`
  if (secs < 3600) return `${Math.floor(secs / 60)}m`
  return `${(secs / 3600).toFixed(1)}h`
}

function formatTime(unix: number) {
  if (!unix) return '—'
  return new Date(unix * 1000).toLocaleString()
}

function maxOnline(samples: { online: number }[]) {
  if (!samples.length) return 0
  return Math.max(...samples.map((s) => s.online))
}
</script>

<template>
  <div class="pp-page">
    <NetworkSubnav />
    <PpPageHeader
      title="Player analytics"
      description="Patch Plan — Plan-inspired analytics across your network. Data is exported locally from each Pumpkin server with a signed snapshot only Pumpkin Patch can read."
    >
      <template #actions>
        <PpButton variant="secondary" size="sm" :loading="isFetching" @click="refetch()">
          Refresh
        </PpButton>
      </template>
    </PpPageHeader>

    <p v-if="isLoading" class="pp-muted">Loading analytics…</p>

    <template v-else-if="overview">
      <PpCard padding="lg" class="intro">
        <p class="pp-muted intro-text">
          Inspired by
          <a
            href="https://www.playeranalytics.net"
            target="_blank"
            rel="noopener"
            class="link"
          >
            Plan (Player Analytics)
          </a>
          . This is a native Pumpkin plugin — not the Java Plan jar — built for Pumpkin Patch
          networks.
        </p>
      </PpCard>

      <div class="server-tabs" data-tauri-drag-region-exclude>
        <button
          type="button"
          class="tab"
          :class="{ 'tab--active': selectedServerId === 'combined' }"
          @click="selectedServerId = 'combined'"
        >
          Combined network
        </button>
        <button
          v-for="s in overview.servers"
          :key="s.server_id"
          type="button"
          class="tab"
          :class="{ 'tab--active': selectedServerId === s.server_id }"
          @click="selectedServerId = s.server_id"
        >
          {{ s.server_name }}
          <PpBadge v-if="s.stale" tone="warning" class="tab-badge">Stale</PpBadge>
        </button>
      </div>

      <div v-if="selectedServerId === 'combined'" class="stats-grid">
        <PpCard padding="md" class="stat">
          <p class="stat-label pp-muted">Online now</p>
          <p class="stat-value">{{ overview.combined_online }}</p>
        </PpCard>
        <PpCard padding="md" class="stat">
          <p class="stat-label pp-muted">Unique players (sum of servers)</p>
          <p class="stat-value">{{ overview.combined_unique_players }}</p>
        </PpCard>
        <PpCard padding="md" class="stat">
          <p class="stat-label pp-muted">Total joins</p>
          <p class="stat-value">{{ overview.combined_total_joins.toLocaleString() }}</p>
        </PpCard>
        <PpCard padding="md" class="stat">
          <p class="stat-label pp-muted">Playtime tracked</p>
          <p class="stat-value">{{ formatDuration(overview.combined_playtime_secs) }}</p>
        </PpCard>
      </div>

      <div v-else-if="selectedSnapshot" class="stats-grid">
        <PpCard padding="md" class="stat">
          <p class="stat-label pp-muted">Online now</p>
          <p class="stat-value">{{ selectedSnapshot.online_now }}</p>
        </PpCard>
        <PpCard padding="md" class="stat">
          <p class="stat-label pp-muted">Peak online</p>
          <p class="stat-value">{{ selectedSnapshot.peak_online }}</p>
        </PpCard>
        <PpCard padding="md" class="stat">
          <p class="stat-label pp-muted">Unique players</p>
          <p class="stat-value">{{ selectedSnapshot.unique_players }}</p>
        </PpCard>
        <PpCard padding="md" class="stat">
          <p class="stat-label pp-muted">Last export</p>
          <p class="stat-value stat-value--sm">{{ formatTime(selectedSnapshot.generated_at_unix) }}</p>
        </PpCard>
      </div>

      <PpCard v-if="selectedServerId === 'combined'" padding="lg" class="section">
        <h3 class="section-title">Per-server snapshot</h3>
        <div class="server-table">
          <div v-for="s in overview.servers" :key="s.server_id" class="server-row">
            <span class="server-name">{{ s.server_name }}</span>
            <span class="pp-muted">{{ s.online_now }} online</span>
            <span class="pp-muted">{{ s.unique_players }} players</span>
            <span class="pp-muted">{{ s.total_joins }} joins</span>
            <PpBadge v-if="s.stale" tone="warning">No recent export</PpBadge>
          </div>
        </div>
        <p class="pp-muted footnote">
          Start servers with Patch Plan installed (auto-deployed on network create) and join with
          players to populate exports. Snapshots refresh about every five minutes while running.
        </p>
      </PpCard>

      <PpCard
        v-if="selectedSnapshot && selectedSnapshot.top_players.length"
        padding="lg"
        class="section"
      >
        <h3 class="section-title">Top players by playtime — {{ selectedSnapshot.server_name }}</h3>
        <ul class="player-list">
          <li v-for="p in selectedSnapshot.top_players" :key="p.uuid" class="player-row">
            <span class="player-name">{{ p.name }}</span>
            <span class="pp-muted">{{ formatDuration(p.playtime_secs) }}</span>
            <span class="pp-muted">{{ p.join_count }} joins</span>
          </li>
        </ul>
      </PpCard>

      <PpCard
        v-if="selectedSnapshot?.activity_samples.length"
        padding="lg"
        class="section"
      >
        <h3 class="section-title">Recent online activity</h3>
        <p class="pp-muted chart-caption">
          Peak sample: {{ maxOnline(selectedSnapshot.activity_samples) }} players
        </p>
        <div class="spark-bars" aria-hidden="true">
          <div
            v-for="(pt, idx) in selectedSnapshot.activity_samples.slice(-48)"
            :key="idx"
            class="spark-bar"
            :style="{
              height: `${Math.max(4, (pt.online / Math.max(1, maxOnline(selectedSnapshot.activity_samples))) * 100)}%`,
            }"
          />
        </div>
      </PpCard>
    </template>
  </div>
</template>

<style scoped>
.intro-text {
  margin: 0;
  font-size: 0.9rem;
  line-height: 1.5;
}
.link {
  color: var(--color-accent-hover);
}
.server-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  margin-bottom: 1rem;
}
.tab {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.45rem 0.85rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg-elevated);
  color: var(--color-text-muted);
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}
.tab--active {
  color: var(--color-accent-hover);
  border-color: rgba(230, 126, 34, 0.45);
  background: var(--color-accent-muted);
}
.tab-badge {
  font-size: 0.65rem;
}
.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(11rem, 1fr));
  gap: 0.75rem;
  margin-bottom: 1rem;
}
.stat-label {
  margin: 0 0 0.35rem;
  font-size: 0.75rem;
}
.stat-value {
  margin: 0;
  font-size: 1.65rem;
  font-weight: 700;
}
.stat-value--sm {
  font-size: 0.95rem;
  font-weight: 600;
}
.section-title {
  margin: 0 0 0.75rem;
  font-size: 1rem;
}
.server-table {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.server-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  align-items: center;
  font-size: 0.85rem;
}
.server-name {
  font-weight: 600;
  min-width: 8rem;
}
.footnote {
  margin: 1rem 0 0;
  font-size: 0.8rem;
}
.player-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}
.player-row {
  display: flex;
  gap: 1rem;
  font-size: 0.85rem;
}
.player-name {
  flex: 1;
  font-weight: 600;
}
.spark-bars {
  display: flex;
  align-items: flex-end;
  gap: 2px;
  height: 4rem;
  margin-top: 0.5rem;
}
.spark-bar {
  flex: 1;
  min-width: 3px;
  background: var(--color-accent);
  border-radius: 2px 2px 0 0;
  opacity: 0.85;
  transition: height 0.2s ease;
}
.chart-caption {
  margin: 0;
  font-size: 0.8rem;
}
</style>
