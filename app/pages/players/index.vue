<script setup lang="ts">
import { useQuery } from '@tanstack/vue-query'
import type { NetworkSummary } from '~/types/patch'

const route = useRoute()
const api = usePatchApi()

const { data: networks } = useQuery({
  queryKey: ['networks'],
  queryFn: () => api.networkList(),
})

const selectedNetworkId = ref<string>(
  typeof route.query.network === 'string' ? route.query.network : '',
)

watch(
  networks,
  (list) => {
    if (!selectedNetworkId.value && list?.length) {
      selectedNetworkId.value = list[0].network.id
    }
  },
  { immediate: true },
)

const { data: players, isLoading, refetch } = useQuery({
  queryKey: ['global-plan-players', selectedNetworkId],
  queryFn: () => api.analyticsPlanPlayersV1(selectedNetworkId.value, undefined, 500),
  enabled: computed(() => Boolean(selectedNetworkId.value)),
})

const profileUuid = ref<string | null>(null)

const { data: profile } = useQuery({
  queryKey: ['global-plan-player', selectedNetworkId, profileUuid],
  queryFn: () => api.analyticsPlanPlayer(selectedNetworkId.value, profileUuid.value!),
  enabled: computed(() => Boolean(selectedNetworkId.value && profileUuid.value)),
})

function formatMs(ms: number) {
  const h = Math.floor(ms / 3_600_000)
  const m = Math.floor((ms % 3_600_000) / 60_000)
  if (h > 0) return `${h}h ${m}m`
  return `${m}m`
}

function pickNetwork(item: NetworkSummary) {
  selectedNetworkId.value = item.network.id
  profileUuid.value = null
  navigateTo({ path: '/players', query: { network: item.network.id } })
}
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div>
        <h1 class="page-title">Players</h1>
        <p class="pp-muted">Plan player index across a network (local Plan database).</p>
      </div>
      <button type="button" class="refresh-btn" @click="refetch()">Refresh</button>
    </header>

    <div v-if="networks?.length" class="network-pick">
      <button
        v-for="item in networks"
        :key="item.network.id"
        type="button"
        class="chip"
        :class="{ 'chip--on': selectedNetworkId === item.network.id }"
        @click="pickNetwork(item)"
      >
        {{ item.network.name }}
      </button>
    </div>

    <p v-if="!networks?.length" class="pp-muted">Create a network first to browse players.</p>
    <p v-else-if="isLoading" class="pp-muted">Loading players…</p>
    <p v-else-if="!players?.data?.length" class="pp-muted">
      Deploy Patch Plan on servers and play — data syncs when you open Analytics → Refresh.
    </p>

    <PpCard v-else padding="lg" class="section">
      <ul class="plan-table">
          <li v-for="p in players.data" :key="p.uuid" class="plan-row">
          <button type="button" class="plan-name" @click="profileUuid = p.uuid">{{ p.name }}</button>
          <span class="pp-muted">{{ p.activity_group }}</span>
          <span>{{ p.activity_index.toFixed(1) }}</span>
          <span class="pp-muted">{{ formatMs(p.active_playtime) }}</span>
          <span class="pp-muted">{{ p.sessions }} sessions</span>
        </li>
      </ul>
    </PpCard>

    <PpCard v-if="profile" padding="lg" class="section profile-card">
      <div class="profile-head">
        <h2 class="section-title">{{ profile.info.name }}</h2>
        <button type="button" class="plan-link" @click="profileUuid = null">Close</button>
      </div>
      <p class="pp-muted">
        Activity {{ profile.info.activity_index.toFixed(1) }} ·
        {{ profile.info.activity_group }} ·
        {{ profile.info.sessions }} sessions ·
        {{ formatMs(profile.info.active_playtime_ms) }} active
      </p>
      <p v-if="profile.stats" class="pp-muted">
        {{ formatMs(profile.stats.total_playtime_ms) }} play · {{ profile.stats.session_count }} sessions ·
        {{ profile.stats.player_kill_count }} PvP kills
      </p>
      <ul v-if="profile.info.geolocation || profile.info.join_address || profile.info.kick_count" class="profile-meta">
        <li v-if="profile.info.geolocation" class="pp-muted">Region: {{ profile.info.geolocation }}</li>
        <li v-if="profile.info.join_address" class="pp-muted">Join: {{ profile.info.join_address }}</li>
        <li v-if="profile.info.kick_count" class="pp-muted">Kicks: {{ profile.info.kick_count }}</li>
      </ul>
      <NuxtLink
        v-if="selectedNetworkId"
        :to="`/networks/${selectedNetworkId}/analytics`"
        class="plan-link"
      >
        Open full analytics
      </NuxtLink>
    </PpCard>
  </div>
</template>

<style scoped>
.page {
  padding: 1.25rem 1.5rem;
  max-width: 56rem;
}
.page-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
  margin-bottom: 1rem;
}
.page-title {
  margin: 0 0 0.25rem;
  font-size: 1.35rem;
}
.refresh-btn {
  padding: 0.45rem 0.85rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg-elevated);
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}
.network-pick {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  margin-bottom: 1rem;
}
.chip {
  padding: 0.35rem 0.65rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg-elevated);
  font-size: 0.8rem;
  cursor: pointer;
  transition: all 0.2s ease;
}
.chip--on {
  border-color: rgba(230, 126, 34, 0.55);
  background: var(--color-accent-muted);
  color: var(--color-accent-hover);
}
.plan-table {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}
.plan-row {
  display: grid;
  grid-template-columns: 1.2fr 0.9fr 0.5fr 0.7fr 0.6fr;
  gap: 0.5rem;
  align-items: center;
  font-size: 0.85rem;
  padding: 0.35rem 0;
  border-bottom: 1px solid var(--color-border-subtle);
}
.plan-name {
  border: none;
  background: none;
  padding: 0;
  text-align: left;
  cursor: pointer;
  color: var(--color-accent-hover);
  font-weight: 600;
}
.profile-meta {
  list-style: none;
  margin: 0.5rem 0 0;
  padding: 0;
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  font-size: 0.85rem;
}
.profile-card {
  margin-top: 1rem;
  border-color: rgba(230, 126, 34, 0.35);
}
.profile-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.plan-link {
  color: var(--color-accent-hover);
  font-weight: 600;
  text-decoration: none;
}
</style>
