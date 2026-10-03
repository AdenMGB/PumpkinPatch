<script setup lang="ts">
import { useQuery } from '@tanstack/vue-query'
import type { NetworkAnalyticsOverview, PlayerDetailRow } from '~/types/patch'

const route = useRoute()
const api = usePatchApi()
const id = computed(() => route.params.id as string)

const selectedServerId = ref<string | 'combined'>('combined')
type Panel = 'overview' | 'players' | 'deaths' | 'advancements' | 'statistics' | 'feed'
const panel = ref<Panel>('overview')
const selectedPlayerUuid = ref<string | null>(null)

const { data: overview, isLoading, refetch, isFetching } = useQuery({
  queryKey: ['analytics', id],
  queryFn: () => api.analyticsNetworkOverview(id.value),
  refetchInterval: 30_000,
})

const selectedSnapshot = computed(() => {
  if (!overview.value || selectedServerId.value === 'combined') return null
  return overview.value.servers.find((s) => s.server_id === selectedServerId.value) ?? null
})

const activePlayers = computed((): PlayerDetailRow[] => {
  if (!overview.value) return []
  if (selectedSnapshot.value) return selectedSnapshot.value.top_players
  const map = new Map<string, PlayerDetailRow>()
  for (const s of overview.value.servers) {
    for (const p of s.top_players) {
      const existing = map.get(p.uuid)
      if (!existing || p.playtime_secs > existing.playtime_secs) {
        map.set(p.uuid, p)
      }
    }
  }
  return [...map.values()].sort((a, b) => b.playtime_secs - a.playtime_secs)
})

const selectedPlayer = computed(() =>
  activePlayers.value.find((p) => p.uuid === selectedPlayerUuid.value) ?? null,
)

const deathsFeed = computed(() => {
  if (!overview.value) return []
  if (selectedSnapshot.value) return selectedSnapshot.value.recent_deaths
  return overview.value.recent_deaths
})

const eventsFeed = computed(() => {
  if (!overview.value) return []
  if (selectedSnapshot.value) return selectedSnapshot.value.recent_events
  return overview.value.recent_events
})

const statsRows = computed(() => {
  if (!overview.value) return []
  if (selectedSnapshot.value) return selectedSnapshot.value.server_totals.top_statistics
  return overview.value.combined_top_statistics
})

const advancementRows = computed(() => {
  const rows: { player: string; id: string; server?: string }[] = []
  const source = selectedSnapshot.value
    ? [selectedSnapshot.value]
    : overview.value?.servers ?? []
  for (const s of source) {
    for (const p of s.top_players) {
      for (const adv of p.recent_advancements) {
        rows.push({ player: p.name, id: adv, server: s.server_name })
      }
    }
  }
  return rows.slice(0, 80)
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

function statLabel(id: string) {
  return id.split(':').pop()?.replace(/_/g, ' ') ?? id
}

const kpi = computed(() => {
  const o = overview.value
  if (!o) return null
  if (selectedSnapshot.value) {
    const s = selectedSnapshot.value
    return {
      online: s.online_now,
      deaths: s.server_totals.total_deaths,
      advancements: s.server_totals.total_advancements,
      mobKills: s.server_totals.total_mob_kills,
      playerKills: s.server_totals.total_player_kills,
      playtime: s.total_playtime_secs,
    }
  }
  return {
    online: o.combined_online,
    deaths: o.combined_deaths,
    advancements: o.combined_advancements,
    mobKills: o.combined_mob_kills,
    playerKills: o.combined_player_kills,
    playtime: o.combined_playtime_secs,
  }
})
</script>

<template>
  <div class="pp-page analytics-page">
    <NetworkSubnav />
    <PpPageHeader
      title="Player analytics"
      description="Patch Plan tracks joins, deaths, advancements, Minecraft statistics, sessions, and live activity — exported locally with HMAC signatures."
    >
      <template #actions>
        <PpButton variant="secondary" size="sm" :loading="isFetching" @click="refetch()">
          Refresh
        </PpButton>
      </template>
    </PpPageHeader>

    <p v-if="isLoading" class="pp-muted">Loading analytics…</p>

    <template v-else-if="overview">
      <div class="server-tabs" data-tauri-drag-region-exclude>
        <button
          type="button"
          class="tab"
          :class="{ 'tab--active': selectedServerId === 'combined' }"
          @click="selectedServerId = 'combined'"
        >
          Combined
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

      <div class="panel-tabs" data-tauri-drag-region-exclude>
        <button
          v-for="p in (
            [
              ['overview', 'Overview'],
              ['players', 'Players'],
              ['deaths', 'Deaths'],
              ['advancements', 'Advancements'],
              ['statistics', 'Statistics'],
              ['feed', 'Live feed'],
            ] as const
          )"
          :key="p[0]"
          type="button"
          class="panel-tab"
          :class="{ 'panel-tab--active': panel === p[0] }"
          @click="panel = p[0]"
        >
          {{ p[1] }}
        </button>
      </div>

      <div v-if="kpi && panel === 'overview'" class="stats-grid">
        <PpCard padding="md" class="stat">
          <p class="stat-label pp-muted">Online now</p>
          <p class="stat-value">{{ kpi.online }}</p>
        </PpCard>
        <PpCard padding="md" class="stat">
          <p class="stat-label pp-muted">Deaths</p>
          <p class="stat-value">{{ kpi.deaths.toLocaleString() }}</p>
        </PpCard>
        <PpCard padding="md" class="stat">
          <p class="stat-label pp-muted">Advancements</p>
          <p class="stat-value">{{ kpi.advancements.toLocaleString() }}</p>
        </PpCard>
        <PpCard padding="md" class="stat">
          <p class="stat-label pp-muted">Mob kills</p>
          <p class="stat-value">{{ kpi.mobKills.toLocaleString() }}</p>
        </PpCard>
        <PpCard padding="md" class="stat">
          <p class="stat-label pp-muted">Player kills</p>
          <p class="stat-value">{{ kpi.playerKills.toLocaleString() }}</p>
        </PpCard>
        <PpCard padding="md" class="stat">
          <p class="stat-label pp-muted">Playtime tracked</p>
          <p class="stat-value stat-value--sm">{{ formatDuration(kpi.playtime) }}</p>
        </PpCard>
      </div>

      <PpCard
        v-if="panel === 'overview' && selectedSnapshot?.activity_samples.length"
        padding="lg"
        class="section"
      >
        <h3 class="section-title">Online activity</h3>
        <div class="spark-bars" aria-hidden="true">
          <div
            v-for="(pt, idx) in selectedSnapshot!.activity_samples.slice(-64)"
            :key="idx"
            class="spark-bar"
            :style="{
              height: `${Math.max(4, (pt.online / Math.max(1, maxOnline(selectedSnapshot!.activity_samples))) * 100)}%`,
            }"
          />
        </div>
      </PpCard>

      <PpCard v-if="panel === 'players'" padding="lg" class="section">
        <h3 class="section-title">Players</h3>
        <div class="player-layout">
          <ul class="player-list">
            <li
              v-for="p in activePlayers"
              :key="p.uuid"
              class="player-row"
              :class="{ 'player-row--active': selectedPlayerUuid === p.uuid }"
            >
              <button type="button" class="player-btn" @click="selectedPlayerUuid = p.uuid">
                <span class="player-name">{{ p.name }}</span>
                <span class="pp-muted">{{ formatDuration(p.playtime_secs) }}</span>
              </button>
            </li>
          </ul>
          <div v-if="selectedPlayer" class="player-detail">
            <h4>{{ selectedPlayer.name }}</h4>
            <dl class="detail-grid">
              <dt>Deaths</dt>
              <dd>{{ selectedPlayer.deaths }}</dd>
              <dt>Mob kills</dt>
              <dd>{{ selectedPlayer.mob_kills }}</dd>
              <dt>Player kills</dt>
              <dd>{{ selectedPlayer.player_kills }}</dd>
              <dt>Advancements</dt>
              <dd>{{ selectedPlayer.advancements }}</dd>
              <dt>Joins</dt>
              <dd>{{ selectedPlayer.join_count }}</dd>
              <dt>Logins</dt>
              <dd>{{ selectedPlayer.login_count }}</dd>
              <dt>Blocks harvested</dt>
              <dd>{{ selectedPlayer.blocks_harvested }}</dd>
              <dt>Recipes</dt>
              <dd>{{ selectedPlayer.recipes_discovered }}</dd>
              <dt>Chat messages</dt>
              <dd>{{ selectedPlayer.chat_messages }}</dd>
              <dt>Commands</dt>
              <dd>{{ selectedPlayer.commands_used }}</dd>
              <dt>Gamemode</dt>
              <dd>{{ selectedPlayer.current_gamemode || '—' }}</dd>
              <dt>Last world</dt>
              <dd>{{ selectedPlayer.last_world || '—' }}</dd>
              <dt>Level</dt>
              <dd>{{ selectedPlayer.xp_level }}</dd>
              <dt>Last seen</dt>
              <dd>{{ formatTime(selectedPlayer.last_seen_unix) }}</dd>
            </dl>
            <h5 v-if="selectedPlayer.top_statistics.length">Top statistics</h5>
            <ul v-if="selectedPlayer.top_statistics.length" class="mini-stats">
              <li v-for="s in selectedPlayer.top_statistics" :key="s.id">
                <span>{{ statLabel(s.id) }}</span>
                <span class="pp-muted">{{ s.total.toLocaleString() }}</span>
              </li>
            </ul>
          </div>
          <p v-else class="pp-muted player-hint">Select a player for full stats.</p>
        </div>
      </PpCard>

      <PpCard v-if="panel === 'deaths'" padding="lg" class="section">
        <h3 class="section-title">Death log</h3>
        <p v-if="!deathsFeed.length" class="pp-muted">No deaths recorded yet.</p>
        <ul v-else class="feed-list">
          <li v-for="(d, i) in deathsFeed" :key="`${d.t}-${i}`" class="feed-row">
            <span class="feed-time pp-muted">{{ formatTime(d.t) }}</span>
            <span class="feed-who">{{ d.player_name }}</span>
            <span v-if="selectedServerId === 'combined'" class="pp-muted">{{ d.server_name }}</span>
            <span class="feed-detail">{{ d.message }}</span>
          </li>
        </ul>
      </PpCard>

      <PpCard v-if="panel === 'advancements'" padding="lg" class="section">
        <h3 class="section-title">Recent advancements</h3>
        <p v-if="!advancementRows.length" class="pp-muted">No advancements yet.</p>
        <ul v-else class="feed-list">
          <li v-for="(a, i) in advancementRows" :key="`${a.id}-${i}`" class="feed-row">
            <span class="feed-who">{{ a.player }}</span>
            <code class="adv-id">{{ a.id }}</code>
            <span v-if="a.server" class="pp-muted">{{ a.server }}</span>
          </li>
        </ul>
      </PpCard>

      <PpCard v-if="panel === 'statistics'" padding="lg" class="section">
        <h3 class="section-title">Minecraft statistics (aggregated)</h3>
        <p v-if="!statsRows.length" class="pp-muted">
          Statistics appear as players trigger in-game stat increments.
        </p>
        <ul v-else class="mini-stats wide">
          <li v-for="s in statsRows" :key="s.id">
            <span>{{ statLabel(s.id) }}</span>
            <span class="pp-muted mono">{{ s.total.toLocaleString() }}</span>
          </li>
        </ul>
      </PpCard>

      <PpCard v-if="panel === 'feed'" padding="lg" class="section">
        <h3 class="section-title">Live event feed</h3>
        <p v-if="!eventsFeed.length" class="pp-muted">Events will show joins, deaths, advancements, commands, and more.</p>
        <ul v-else class="feed-list">
          <li v-for="(e, i) in eventsFeed.slice(0, 120)" :key="`${e.t}-${i}`" class="feed-row">
            <span class="feed-time pp-muted">{{ formatTime(e.t) }}</span>
            <PpBadge tone="neutral">{{ e.kind }}</PpBadge>
            <span class="feed-who">{{ e.player_name }}</span>
            <span v-if="selectedServerId === 'combined'" class="pp-muted">{{ e.server_name }}</span>
            <span class="feed-detail">{{ e.detail }}</span>
          </li>
        </ul>
      </PpCard>
    </template>
  </div>
</template>

<style scoped>
.server-tabs,
.panel-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  margin-bottom: 0.75rem;
}
.tab,
.panel-tab {
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
.tab--active,
.panel-tab--active {
  color: var(--color-accent-hover);
  border-color: rgba(230, 126, 34, 0.45);
  background: var(--color-accent-muted);
}
.tab-badge {
  font-size: 0.65rem;
}
.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(10rem, 1fr));
  gap: 0.75rem;
  margin-bottom: 1rem;
}
.stat-label {
  margin: 0 0 0.35rem;
  font-size: 0.75rem;
}
.stat-value {
  margin: 0;
  font-size: 1.5rem;
  font-weight: 700;
}
.stat-value--sm {
  font-size: 1rem;
}
.section-title {
  margin: 0 0 0.75rem;
  font-size: 1rem;
}
.spark-bars {
  display: flex;
  align-items: flex-end;
  gap: 2px;
  height: 4.5rem;
}
.spark-bar {
  flex: 1;
  min-width: 3px;
  background: var(--color-accent);
  border-radius: 2px 2px 0 0;
}
.player-layout {
  display: grid;
  grid-template-columns: minmax(12rem, 16rem) 1fr;
  gap: 1rem;
}
@media (max-width: 768px) {
  .player-layout {
    grid-template-columns: 1fr;
  }
}
.player-list {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 24rem;
  overflow: auto;
}
.player-row--active .player-btn {
  background: var(--color-accent-muted);
}
.player-btn {
  width: 100%;
  display: flex;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.45rem 0.6rem;
  border: none;
  border-radius: var(--radius-md);
  background: transparent;
  color: inherit;
  cursor: pointer;
  text-align: left;
  transition: all 0.2s ease;
}
.player-btn:hover {
  background: var(--color-surface-hover);
}
.player-name {
  font-weight: 600;
}
.player-detail h4 {
  margin: 0 0 0.75rem;
}
.player-detail h5 {
  margin: 1rem 0 0.5rem;
  font-size: 0.9rem;
}
.detail-grid {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0.35rem 1rem;
  font-size: 0.85rem;
}
.detail-grid dt {
  color: var(--color-text-muted);
}
.detail-grid dd {
  margin: 0;
}
.mini-stats {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  font-size: 0.85rem;
}
.mini-stats.wide li {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
}
.mono {
  font-family: ui-monospace, monospace;
}
.feed-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.feed-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: baseline;
  font-size: 0.85rem;
  padding: 0.35rem 0;
  border-bottom: 1px solid var(--color-border-subtle);
}
.feed-time {
  font-size: 0.75rem;
  min-width: 9rem;
}
.feed-who {
  font-weight: 600;
}
.feed-detail {
  flex: 1;
  min-width: 8rem;
}
.adv-id {
  font-size: 0.8rem;
}
.player-hint {
  align-self: center;
}
</style>
