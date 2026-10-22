<script setup lang="ts">
import { useMutation, useQuery } from '@tanstack/vue-query'
import type { NetworkAnalyticsOverview, PlayerDetailRow } from '~/types/patch'

const route = useRoute()
const api = usePatchApi()
const id = computed(() => route.params.id as string)

const selectedServerId = ref<string | 'combined'>('combined')
type Panel =
  | 'overview'
  | 'players'
  | 'sessions'
  | 'commands'
  | 'performance'
  | 'pvp'
  | 'worlds'
  | 'geo'
  | 'query'
  | 'deaths'
  | 'advancements'
  | 'statistics'
  | 'feed'
const panel = ref<Panel>('overview')
const selectedPlayerUuid = ref<string | null>(null)
const planProfileUuid = ref<string | null>(null)
const installPlanMessage = ref<string | null>(null)
const installPlanBannerError = ref(false)

const regUsername = ref('')
const regPassword = ref('')
const regCode = ref<string | null>(null)
const regExpires = ref<number | null>(null)
const regError = ref<string | null>(null)
const regLoading = ref(false)

const { data: planWebUsers, refetch: refetchWebUsers } = useQuery({
  queryKey: ['plan-web-users'],
  queryFn: () => api.planWebUsersList(),
})

async function submitPlanRegistration() {
  regError.value = null
  regCode.value = null
  regLoading.value = true
  try {
    const res = await api.planAuthRegisterStart(id.value, regUsername.value.trim(), regPassword.value)
    regCode.value = res.code
    regExpires.value = res.expires_ms
    refetchWebUsers()
  } catch (e: unknown) {
    regError.value = e instanceof Error ? e.message : 'Registration failed'
  } finally {
    regLoading.value = false
  }
}

const { data: networkSummary } = useQuery({
  queryKey: ['network', id],
  queryFn: () => api.networkGet(id.value),
})

const backendServerCount = computed(
  () => networkSummary.value?.servers.filter((s) => s.role === 'backend').length ?? 0,
)

const installPlanMutation = useMutation({
  mutationFn: () => api.analyticsInstallPlanOnBackends(id.value),
  onSuccess: (results) => {
    if (!results.length) {
      installPlanBannerError.value = true
      installPlanMessage.value = 'This network has no backend servers to install on.'
      return
    }
    const ok = results.filter((r) => r.success)
    const failed = results.filter((r) => !r.success)
    installPlanBannerError.value = failed.length > 0
    if (failed.length) {
      installPlanMessage.value = `Installed on ${ok.length} of ${results.length} backends. ${failed
        .map((f) => `${f.server_name}: ${f.detail}`)
        .join(' · ')}`
    } else {
      installPlanMessage.value = `Patch Plan installed on ${ok.length} backend server(s). Restart any running backends so Pumpkin loads the WASM plugin.`
    }
    refetch()
  },
  onError: (err: Error) => {
    installPlanBannerError.value = true
    installPlanMessage.value = err.message ?? 'Install failed'
  },
})

const { data: overview, isLoading, refetch, isFetching } = useQuery({
  queryKey: ['analytics', id],
  queryFn: () => api.analyticsNetworkOverview(id.value),
  refetchInterval: 30_000,
})

const planServerId = computed(() =>
  selectedServerId.value === 'combined' ? undefined : selectedServerId.value,
)

const { data: planSessionsV1 } = useQuery({
  queryKey: ['analytics-plan-sessions-v1', id, planServerId],
  queryFn: () => api.analyticsPlanSessionsV1(id.value, planServerId.value),
  enabled: computed(() => panel.value === 'sessions'),
})

const planSessions = computed(() => planSessionsV1.value?.most_recent_sessions ?? [])

const { data: planCommands } = useQuery({
  queryKey: ['analytics-plan-commands', id, planServerId],
  queryFn: () => api.analyticsPlanCommands(id.value, planServerId.value),
  enabled: computed(() => panel.value === 'commands'),
})

const { data: planPlayers } = useQuery({
  queryKey: ['analytics-plan-players', id, planServerId],
  queryFn: () => api.analyticsPlanPlayersTable(id.value, planServerId.value),
  enabled: computed(() => panel.value === 'players'),
})

const { data: planGraph } = useQuery({
  queryKey: ['analytics-plan-graph', id, planServerId],
  queryFn: () => api.analyticsPlanGraphPlayersOnline(id.value, planServerId.value),
  enabled: computed(() => panel.value === 'performance'),
})

const { data: planVersus } = useQuery({
  queryKey: ['analytics-plan-pvp', id, planServerId],
  queryFn: () => api.analyticsPlanPlayerVersus(id.value, planServerId.value),
  enabled: computed(() => panel.value === 'pvp'),
})

const { data: planKills } = useQuery({
  queryKey: ['analytics-plan-kills', id, planServerId],
  queryFn: () => api.analyticsPlanKills(id.value, planServerId.value),
  enabled: computed(() => panel.value === 'pvp'),
})

const { data: sessionsOverview } = useQuery({
  queryKey: ['analytics-plan-sessions-overview', id, planServerId],
  queryFn: () => api.analyticsPlanSessionsOverview(id.value, planServerId.value),
  enabled: computed(() => panel.value === 'sessions'),
})

const { data: worldPie } = useQuery({
  queryKey: ['analytics-plan-world-pie', id, planServerId],
  queryFn: () => api.analyticsPlanGraph(id.value, 'worldPie', planServerId.value),
  enabled: computed(() => panel.value === 'worlds'),
})

const { data: pingTable } = useQuery({
  queryKey: ['analytics-plan-ping', id, planServerId],
  queryFn: () => api.analyticsPlanPingTable(id.value, planServerId.value),
  enabled: computed(() => panel.value === 'geo'),
})

const { data: geoGraph } = useQuery({
  queryKey: ['analytics-plan-geo-graph', id, planServerId],
  queryFn: () => api.analyticsPlanGraph(id.value, 'geolocation', planServerId.value),
  enabled: computed(() => panel.value === 'geo'),
})

const { data: joinAddresses } = useQuery({
  queryKey: ['analytics-plan-join-addresses', id, planServerId],
  queryFn: () => api.analyticsPlanJoinAddresses(id.value, planServerId.value),
  enabled: computed(() => panel.value === 'query'),
})

const { data: planFilters } = useQuery({
  queryKey: ['analytics-plan-filters', id, planServerId],
  queryFn: () => api.analyticsPlanFilters(id.value, planServerId.value),
  enabled: computed(() => panel.value === 'query'),
})

const queryActivityGroups = ref<string[]>([])
const queryJoinAddresses = ref<string[]>([])
const queryMinActivity = ref<number | null>(null)
const queryMinSessions = ref<number | null>(null)
const queryRegisteredAfter = ref<string>('')
const queryRegisteredBefore = ref<string>('')
const queryViewAfter = ref<string>('')
const queryViewBefore = ref<string>('')
const queryLastSeenAfter = ref<string>('')
const queryLastSeenBefore = ref<string>('')
const queryPlayedAfter = ref<string>('')
const queryPlayedBefore = ref<string>('')
const queryGeolocations = ref<string[]>([])
const planQueryResult = ref<Awaited<ReturnType<typeof api.analyticsPlanQuery>> | null>(null)
const planQueryLoading = ref(false)
const queryBypassCache = ref(false)
const queryShareCopied = ref(false)

async function runPlanQuery() {
  planQueryLoading.value = true
  try {
    planQueryResult.value = await api.analyticsPlanQuery(
      id.value,
      {
        activity_groups: queryActivityGroups.value,
        join_addresses: queryJoinAddresses.value,
        min_activity_index: queryMinActivity.value,
        max_activity_index: null,
        min_sessions: queryMinSessions.value,
        registered_after_ms: queryRegisteredAfter.value
          ? new Date(queryRegisteredAfter.value).getTime()
          : null,
        registered_before_ms: queryRegisteredBefore.value
          ? new Date(queryRegisteredBefore.value).getTime()
          : null,
        geolocations: queryGeolocations.value,
        bypass_cache: queryBypassCache.value,
        view_after_ms: queryViewAfter.value ? new Date(queryViewAfter.value).getTime() : null,
        view_before_ms: queryViewBefore.value ? new Date(queryViewBefore.value).getTime() : null,
        last_seen_after_ms: queryLastSeenAfter.value
          ? new Date(queryLastSeenAfter.value).getTime()
          : null,
        last_seen_before_ms: queryLastSeenBefore.value
          ? new Date(queryLastSeenBefore.value).getTime()
          : null,
        played_after_ms: queryPlayedAfter.value ? new Date(queryPlayedAfter.value).getTime() : null,
        played_before_ms: queryPlayedBefore.value
          ? new Date(queryPlayedBefore.value).getTime()
          : null,
        played_server_uuids: [],
      },
      planServerId.value,
    )
    if (planQueryResult.value?.cache_token) {
      await navigateTo({
        path: route.path,
        query: { ...route.query, q: planQueryResult.value.cache_token },
      })
    }
  } finally {
    planQueryLoading.value = false
  }
}

async function loadQueryFromToken(token: string) {
  panel.value = 'query'
  planQueryLoading.value = true
  try {
    planQueryResult.value = await api.analyticsPlanQueryByToken(id.value, token)
  } finally {
    planQueryLoading.value = false
  }
}

function querySharePath(token: string) {
  return `${route.path}?q=${encodeURIComponent(token)}`
}

async function copyQueryShareLink() {
  const token = planQueryResult.value?.cache_token
  if (!token) return
  const link = `${window.location.origin}${querySharePath(token)}`
  await navigator.clipboard.writeText(link)
  queryShareCopied.value = true
  window.setTimeout(() => {
    queryShareCopied.value = false
  }, 2000)
}

watch(
  () => route.query.q,
  (q) => {
    if (typeof q === 'string' && q.length > 0) loadQueryFromToken(q)
  },
  { immediate: true },
)

function toggleQueryGroup(group: string) {
  const i = queryActivityGroups.value.indexOf(group)
  if (i >= 0) queryActivityGroups.value = queryActivityGroups.value.filter((g) => g !== group)
  else queryActivityGroups.value = [...queryActivityGroups.value, group]
}

function toggleQueryGeo(geo: string) {
  const i = queryGeolocations.value.indexOf(geo)
  if (i >= 0) queryGeolocations.value = queryGeolocations.value.filter((g) => g !== geo)
  else queryGeolocations.value = [...queryGeolocations.value, geo]
}

function toggleQueryJoin(addr: string) {
  const i = queryJoinAddresses.value.indexOf(addr)
  if (i >= 0) queryJoinAddresses.value = queryJoinAddresses.value.filter((a) => a !== addr)
  else queryJoinAddresses.value = [...queryJoinAddresses.value, addr]
}

const { data: planCpuRam } = useQuery({
  queryKey: ['analytics-plan-cpu-ram', id, planServerId],
  queryFn: () => api.analyticsPlanGraph(id.value, 'cpuRam', planServerId.value),
  enabled: computed(() => panel.value === 'performance'),
})

const { data: planTpsGraph } = useQuery({
  queryKey: ['analytics-plan-tps', id, planServerId],
  queryFn: () => api.analyticsPlanGraph(id.value, 'tps', planServerId.value),
  enabled: computed(() => panel.value === 'performance'),
})

const { data: activityGraph } = useQuery({
  queryKey: ['analytics-plan-activity-pie', id, planServerId],
  queryFn: () => api.analyticsPlanGraph(id.value, 'activity', planServerId.value),
  enabled: computed(() => panel.value === 'overview'),
})

const activityPie = computed(() => {
  const series = activityGraph.value?.activity_pie_series as { name: string; y: number }[] | undefined
  return series ?? []
})

const { data: planUniqueNew } = useQuery({
  queryKey: ['analytics-plan-unique-new', id, planServerId],
  queryFn: () => api.analyticsPlanGraph(id.value, 'uniqueAndNew', planServerId.value),
  enabled: computed(() => panel.value === 'overview'),
})

const { data: planHourlyUnique } = useQuery({
  queryKey: ['analytics-plan-hourly-unique', id, planServerId],
  queryFn: () => api.analyticsPlanGraph(id.value, 'hourlyUniqueAndNew', planServerId.value),
  enabled: computed(() => panel.value === 'overview'),
})

const { data: planServerCalendar } = useQuery({
  queryKey: ['analytics-plan-calendar', id, planServerId],
  queryFn: () => api.analyticsPlanGraph(id.value, 'serverCalendar', planServerId.value),
  enabled: computed(() => panel.value === 'overview'),
})

const { data: planJoinByDay } = useQuery({
  queryKey: ['analytics-plan-join-by-day', id, planServerId],
  queryFn: () => api.analyticsPlanGraph(id.value, 'joinAddressesByDay', planServerId.value),
  enabled: computed(() => panel.value === 'overview' || panel.value === 'query'),
})

const joinAddressTotals = computed(() => {
  const points = planJoinByDay.value?.join_addresses_by_day as
    | { join_address: string; joins: number }[]
    | undefined
  if (!points?.length) return []
  const totals = new Map<string, number>()
  for (const p of points) {
    totals.set(p.join_address, (totals.get(p.join_address) ?? 0) + p.joins)
  }
  return [...totals.entries()]
    .map(([join_address, joins]) => ({ join_address, joins }))
    .sort((a, b) => b.joins - a.joins)
    .slice(0, 12)
})

const { data: planWorldLoad } = useQuery({
  queryKey: ['analytics-plan-world-load', id, planServerId],
  queryFn: () => api.analyticsPlanGraph(id.value, 'worldLoad', planServerId.value),
  enabled: computed(() => panel.value === 'performance'),
})

const { data: planDiskGraph } = useQuery({
  queryKey: ['analytics-plan-disk', id, planServerId],
  queryFn: () => api.analyticsPlanGraph(id.value, 'disk', planServerId.value),
  enabled: computed(() => panel.value === 'performance'),
})

const tpsSpark = computed(() => {
  const tps = planTpsGraph.value?.tps as [number, number][] | undefined
  if (!tps?.length) return []
  const max = Math.max(...tps.map((p) => p[1]), 1)
  return tps.slice(-96).map((p) => ({ v: p[1] / max }))
})

const { data: planPunchCard } = useQuery({
  queryKey: ['analytics-plan-punchcard', id, planServerId],
  queryFn: () => api.analyticsPlanGraph(id.value, 'punchCard', planServerId.value),
  enabled: computed(() => panel.value === 'performance'),
})

const cpuSpark = computed(() => {
  const cpu = planCpuRam.value?.cpu as [number, number][] | undefined
  if (!cpu?.length) return []
  const max = Math.max(...cpu.map((p) => p[1]), 1)
  return cpu.slice(-96).map((p) => ({ t: p[0], v: p[1] / max }))
})

const ramSpark = computed(() => {
  const ram = planCpuRam.value?.ram as [number, number][] | undefined
  if (!ram?.length) return []
  const max = Math.max(...ram.map((p) => p[1]), 1)
  return ram.slice(-96).map((p) => ({ t: p[0], v: p[1] / max }))
})

const punchGrid = computed(() => {
  const raw = planPunchCard.value?.punchcard as { x: number; y: number; z: number }[] | undefined
  if (!raw?.length) return null
  const maxZ = Math.max(...raw.map((p) => p.z), 1)
  const cells = new Map<string, number>()
  for (const p of raw) cells.set(`${p.y}-${p.x}`, p.z / maxZ)
  return { cells, maxZ }
})

const { data: planProfile } = useQuery({
  queryKey: ['analytics-plan-player', id, planProfileUuid],
  queryFn: () => api.analyticsPlanPlayer(id.value, planProfileUuid.value!),
  enabled: computed(() => Boolean(planProfileUuid.value)),
})

const { data: onlineOverview } = useQuery({
  queryKey: ['analytics-plan-online-overview', id, planServerId],
  queryFn: () => api.analyticsPlanOnlineOverview(id.value, planServerId.value),
  enabled: computed(() => panel.value === 'overview'),
})

const { data: playerbaseOverview } = useQuery({
  queryKey: ['analytics-plan-playerbase-overview', id, planServerId],
  queryFn: () => api.analyticsPlanPlayerbaseOverview(id.value, planServerId.value),
  enabled: computed(() => panel.value === 'overview'),
})

const { data: perfOverview } = useQuery({
  queryKey: ['analytics-plan-perf-overview', id, planServerId],
  queryFn: () => api.analyticsPlanPerformanceOverview(id.value, planServerId.value),
  enabled: computed(() => panel.value === 'performance'),
})

const { data: planRetention } = useQuery({
  queryKey: ['analytics-plan-retention', id, planServerId],
  queryFn: () => api.analyticsPlanRetention(id.value, planServerId.value),
  enabled: computed(() => panel.value === 'query'),
})

const selectedSnapshot = computed(() => {
  if (!overview.value || selectedServerId.value === 'combined') return null
  return overview.value.servers.find((s) => s.server_id === selectedServerId.value) ?? null
})

const activitySamples = computed(() => {
  if (!overview.value) return []
  if (selectedSnapshot.value) return selectedSnapshot.value.activity_samples
  const buckets = new Map<number, number>()
  for (const s of overview.value.servers) {
    for (const pt of s.activity_samples) {
      buckets.set(pt.t, (buckets.get(pt.t) ?? 0) + pt.online)
    }
  }
  return [...buckets.entries()]
    .sort((a, b) => a[0] - b[0])
    .map(([t, online]) => ({ t, online }))
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

function formatMs(ms: number) {
  return formatDuration(Math.floor(ms / 1000))
}

function formatEpochMs(ms: number) {
  if (!ms) return '—'
  return new Date(ms).toLocaleString()
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

const exportIssues = computed(() => {
  const servers = overview.value?.servers ?? []
  return servers.filter((s) =>
    s.export_status === 'missing' || s.export_status === 'invalid',
  )
})

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
        <PpButton
          variant="secondary"
          size="sm"
          :loading="installPlanMutation.isPending.value"
          :disabled="backendServerCount === 0"
          @click="installPlanMutation.mutate()"
        >
          Install Plan on backends
        </PpButton>
        <PpButton variant="secondary" size="sm" :loading="isFetching" @click="refetch()">
          Refresh
        </PpButton>
      </template>
    </PpPageHeader>

    <p
      v-if="installPlanMessage"
      class="install-banner"
      :class="{ 'install-banner--error': installPlanBannerError }"
    >
      {{ installPlanMessage }}
    </p>

    <PpCard padding="md" class="plan-register-card" data-tauri-drag-region-exclude>
      <h3 class="section-title">Plan web access</h3>
      <p class="pp-muted">
        Create a staff or player web account (indepth.md §5.2). After you get a code, run
        <code class="pp-code">/plan register &lt;code&gt;</code> in-game, then trigger analytics sync.
      </p>
      <div class="plan-register-form">
        <label>
          Username
          <input v-model="regUsername" type="text" autocomplete="username" />
        </label>
        <label>
          Password
          <input v-model="regPassword" type="password" autocomplete="new-password" />
        </label>
        <PpButton size="sm" :loading="regLoading" @click="submitPlanRegistration">
          Request registration code
        </PpButton>
      </div>
      <p v-if="regError" class="install-banner install-banner--error">{{ regError }}</p>
      <div v-if="planWebUsers?.length" class="plan-web-users">
        <h4 class="section-title">Registered web users</h4>
        <table class="plan-users-table">
          <thead>
            <tr>
              <th>User</th>
              <th>Group</th>
              <th>Linked UUID</th>
              <th />
            </tr>
          </thead>
          <tbody>
            <tr v-for="u in planWebUsers" :key="u.username">
              <td>{{ u.username }}</td>
              <td>
                <select
                  :value="u.group_name"
                  @change="
                    api
                      .planWebUserSetGroup(u.username, ($event.target as HTMLSelectElement).value)
                      .then(() => refetchWebUsers())
                  "
                >
                  <option value="admin">admin</option>
                  <option value="staff">staff</option>
                  <option value="player">player</option>
                </select>
              </td>
              <td class="pp-muted">{{ u.linked_to_uuid || '—' }}</td>
              <td>
                <PpButton
                  variant="ghost"
                  size="sm"
                  @click="api.planWebUserLogout(u.username).then(() => refetchWebUsers())"
                >
                  Logout sessions
                </PpButton>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div v-if="regCode" class="plan-register-code">
        <p>
          Your code:
          <strong class="pp-code">{{ regCode }}</strong>
        </p>
        <p class="pp-muted">
          Valid until {{ regExpires ? new Date(regExpires).toLocaleString() : '—' }}. Use
          <code class="pp-code">/plan register {{ regCode }}</code> on a server with Patch Plan.
        </p>
      </div>
    </PpCard>

    <PpCard
      v-if="exportIssues.length"
      padding="md"
      class="export-warning"
      data-tauri-drag-region-exclude
    >
      <h3 class="export-warning__title">Patch Plan export not ready</h3>
      <p class="pp-muted export-warning__lead">
        The UI reads <code class="pp-code">plugins/data/PatchPlan/export.json</code> from each
        server. Config alone is not enough — the WASM plugin must load and write signed exports.
      </p>
      <ul class="export-warning__list">
        <li v-for="s in exportIssues" :key="s.server_id || s.server_name">
          <strong>{{ s.server_name }}</strong>
          <span class="pp-muted">({{ s.export_status }})</span>
          <span v-if="s.export_detail"> — {{ s.export_detail }}</span>
        </li>
      </ul>
    </PpCard>

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
              ['sessions', 'Sessions'],
              ['commands', 'Commands'],
              ['performance', 'Performance'],
              ['pvp', 'PvP & PvE'],
              ['worlds', 'World playtime'],
              ['geo', 'Geo & ping'],
              ['query', 'Query & joins'],
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

      <PpCard v-if="panel === 'overview' && activityPie.length" padding="lg" class="section">
        <h3 class="section-title">Activity groups</h3>
        <ul class="mini-stats wide">
          <li v-for="slice in activityPie" :key="slice.name">
            <span>{{ slice.name }}</span>
            <span class="pp-muted">{{ slice.y }} players</span>
          </li>
        </ul>
      </PpCard>

      <div v-if="panel === 'overview' && (onlineOverview || playerbaseOverview)" class="stats-grid">
        <PpCard v-if="onlineOverview" padding="md" class="stat">
          <p class="stat-label pp-muted">Unique (30d)</p>
          <p class="stat-value stat-value--sm">{{ onlineOverview.numbers.unique_players_30d }}</p>
        </PpCard>
        <PpCard v-if="onlineOverview" padding="md" class="stat">
          <p class="stat-label pp-muted">New players (7d)</p>
          <p class="stat-value stat-value--sm">{{ onlineOverview.numbers.new_players_7d }}</p>
        </PpCard>
        <PpCard v-if="onlineOverview" padding="md" class="stat">
          <p class="stat-label pp-muted">New retention (30d)</p>
          <p class="stat-value stat-value--sm">
            {{ onlineOverview.numbers.new_players_retention_30d.toFixed(0) }}%
          </p>
        </PpCard>
        <PpCard v-if="playerbaseOverview" padding="md" class="stat">
          <p class="stat-label pp-muted">Avg activity index</p>
          <p class="stat-value stat-value--sm">
            {{ playerbaseOverview.insights.avg_activity_index.toFixed(1) }}
          </p>
        </PpCard>
        <PpCard v-if="playerbaseOverview" padding="md" class="stat">
          <p class="stat-label pp-muted">Playerbase trend</p>
          <p class="stat-value stat-value--sm">
            {{ playerbaseOverview.trends.total_players_trend >= 0 ? '+' : ''
            }}{{ playerbaseOverview.trends.total_players_trend.toFixed(0) }}%
          </p>
        </PpCard>
        <PpCard v-if="playerbaseOverview" padding="md" class="stat">
          <p class="stat-label pp-muted">Regular players</p>
          <p class="stat-value stat-value--sm">{{ playerbaseOverview.trends.regular_players_now }}</p>
        </PpCard>
      </div>

      <PpCard
        v-if="panel === 'overview' && (planUniqueNew?.unique_and_new as unknown[])?.length"
        padding="lg"
        class="section"
      >
        <h3 class="section-title">Unique & new players (daily)</h3>
        <ul class="mini-stats wide">
          <li
            v-for="(row, idx) in (planUniqueNew?.unique_and_new as [number, number, number][]).slice(-14)"
            :key="idx"
          >
            <span>{{ new Date(row[0]).toLocaleDateString() }}</span>
            <span class="pp-muted">unique {{ row[1] }} · new {{ row[2] }}</span>
          </li>
        </ul>
      </PpCard>

      <PpCard
        v-if="panel === 'overview' && (planHourlyUnique?.hourly_unique_and_new as unknown[])?.length"
        padding="lg"
        class="section"
      >
        <h3 class="section-title">Unique players (hourly, recent)</h3>
        <ul class="mini-stats wide">
          <li
            v-for="(row, idx) in (planHourlyUnique?.hourly_unique_and_new as [number, number, number][]).slice(-24)"
            :key="idx"
          >
            <span>{{ new Date(row[0]).toLocaleString() }}</span>
            <span class="pp-muted">unique {{ row[1] }}</span>
          </li>
        </ul>
      </PpCard>

      <PpCard
        v-if="panel === 'overview' && (planServerCalendar?.calendar_events as unknown[])?.length"
        padding="lg"
        class="section"
      >
        <h3 class="section-title">Server calendar (daily)</h3>
        <ul class="mini-stats wide">
          <li
            v-for="(ev, idx) in (planServerCalendar?.calendar_events as { title: string; start: number; extendedProps?: { sessions?: number; playtime_ms?: number } }[]).slice(-21)"
            :key="idx"
          >
            <span>{{ new Date(ev.start).toLocaleDateString() }}</span>
            <span class="pp-muted">
              {{ ev.title }}
              <template v-if="ev.extendedProps?.sessions"> · {{ ev.extendedProps.sessions }} sessions</template>
              <template v-if="ev.extendedProps?.playtime_ms">
                · {{ formatMs(ev.extendedProps.playtime_ms) }}
              </template>
            </span>
          </li>
        </ul>
      </PpCard>

      <PpCard v-if="panel === 'overview' && joinAddressTotals.length" padding="lg" class="section">
        <h3 class="section-title">Join addresses by day (totals)</h3>
        <ul class="mini-stats wide">
          <li v-for="j in joinAddressTotals" :key="j.join_address">
            <span class="mono">{{ j.join_address }}</span>
            <span class="pp-muted">{{ j.joins }} session starts</span>
          </li>
        </ul>
      </PpCard>

      <PpCard
        v-if="panel === 'overview' && activitySamples.length"
        padding="lg"
        class="section"
      >
        <h3 class="section-title">Online activity</h3>
        <div class="spark-bars" aria-hidden="true">
          <div
            v-for="(pt, idx) in activitySamples.slice(-64)"
            :key="idx"
            class="spark-bar"
            :style="{
              height: `${Math.max(4, (pt.online / Math.max(1, maxOnline(activitySamples))) * 100)}%`,
            }"
          />
        </div>
      </PpCard>

      <PpCard v-if="panel === 'players'" padding="lg" class="section">
        <h3 class="section-title">Players (Plan index)</h3>
        <p v-if="!planPlayers?.length" class="pp-muted">
          Play a session on a server with Patch Plan deployed — data syncs into the local Plan database on refresh.
        </p>
        <ul v-else class="plan-table">
          <li v-for="p in planPlayers" :key="p.uuid" class="plan-table__row">
            <button type="button" class="feed-who plan-link" @click="planProfileUuid = p.uuid">
              {{ p.name }}
            </button>
            <PpBadge tone="neutral">{{ p.activity_group }}</PpBadge>
            <span class="pp-muted">PAI {{ p.activity_index.toFixed(2) }}</span>
            <span class="pp-muted">{{ formatMs(p.active_playtime_ms) }} active</span>
            <span class="pp-muted">{{ p.sessions }} sessions</span>
          </li>
        </ul>
        <h3 class="section-title section-title--sub">Live export snapshot</h3>
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
                <span class="player-meta">
                  <PpBadge
                    v-if="p.activity_group"
                    tone="neutral"
                    class="activity-badge"
                  >
                    {{ p.activity_group }}
                  </PpBadge>
                  <span class="pp-muted">{{ formatDuration(p.playtime_secs) }}</span>
                </span>
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
              <dt>Activity index</dt>
              <dd>
                {{ selectedPlayer.activity_index?.toFixed(2) ?? '—' }}
                <span v-if="selectedPlayer.activity_group" class="pp-muted">
                  ({{ selectedPlayer.activity_group }})
                </span>
              </dd>
              <dt>AFK time</dt>
              <dd>{{ formatDuration(selectedPlayer.afk_time_secs ?? 0) }}</dd>
              <dt>Join address</dt>
              <dd>{{ selectedPlayer.join_address || '—' }}</dd>
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

      <PpCard v-if="panel === 'sessions'" padding="lg" class="section">
        <h3 class="section-title">Sessions overview</h3>
        <div v-if="sessionsOverview" class="stats-grid stats-grid--compact">
          <PpCard padding="md" class="stat">
            <p class="stat-label pp-muted">Total playtime</p>
            <p class="stat-value stat-value--sm">
              {{ formatMs(sessionsOverview.insights.total_playtime_ms) }}
            </p>
          </PpCard>
          <PpCard padding="md" class="stat">
            <p class="stat-label pp-muted">AFK</p>
            <p class="stat-value stat-value--sm">
              {{ sessionsOverview.insights.afk_time_perc.toFixed(1) }}%
            </p>
          </PpCard>
          <PpCard padding="md" class="stat">
            <p class="stat-label pp-muted">Top gamemode</p>
            <p class="stat-value stat-value--sm">
              {{ sessionsOverview.insights.most_active_gamemode }}
            </p>
          </PpCard>
        </div>
        <h4 class="section-sub">Recent sessions (Plan /v1/sessions)</h4>
        <p v-if="planSessionsV1" class="pp-muted">
          {{ planSessionsV1.sessions.length }} sessions in store · showing
          {{ planSessions.length }} most recent
        </p>
        <p v-if="!planSessions.length" class="pp-muted">No completed sessions in the Plan database yet.</p>
        <ul v-else class="feed-list">
          <li v-for="(s, i) in planSessions" :key="`${s.start}-${i}`" class="feed-row">
            <span class="feed-who">{{ s.player_name }}</span>
            <span class="pp-muted">{{ s.server_name }}</span>
            <span class="feed-time pp-muted">{{ formatEpochMs(s.start) }} → {{ formatEpochMs(s.end) }}</span>
            <span class="pp-muted">{{ formatMs(s.length) }} (AFK {{ formatMs(s.afk_time) }})</span>
            <span v-if="s.first_session" class="accent-text text-xs font-medium">First session</span>
            <span v-if="s.join_address" class="pp-muted">{{ s.join_address }}</span>
            <span
              v-if="s.world_series?.length"
              class="pp-muted text-xs"
            >
              Worlds: {{ s.world_series.map((w) => w.name).join(', ') }}
            </span>
          </li>
        </ul>
      </PpCard>

      <PpCard v-if="panel === 'commands'" padding="lg" class="section">
        <h3 class="section-title">Command usage</h3>
        <p v-if="!planCommands?.length" class="pp-muted">Commands are counted when players run `/` commands in-game.</p>
        <ul v-else class="mini-stats wide">
          <li v-for="c in planCommands" :key="c.command">
            <span><code>/{{ c.command }}</code></span>
            <span class="pp-muted mono">{{ c.times_used.toLocaleString() }}</span>
          </li>
        </ul>
      </PpCard>

      <PpCard v-if="panel === 'performance' && perfOverview" padding="lg" class="section">
        <h3 class="section-title">Performance overview (Plan)</h3>
        <div class="stats-grid stats-grid--compact">
          <PpCard padding="md" class="stat">
            <p class="stat-label pp-muted">Avg TPS (30d)</p>
            <p class="stat-value stat-value--sm">{{ perfOverview.numbers.tps_30d.toFixed(1) }}</p>
          </PpCard>
          <PpCard padding="md" class="stat">
            <p class="stat-label pp-muted">Low TPS spikes (7d)</p>
            <p class="stat-value stat-value--sm">{{ perfOverview.numbers.low_tps_spikes_7d }}</p>
          </PpCard>
          <PpCard padding="md" class="stat">
            <p class="stat-label pp-muted">Uptime (30d)</p>
            <p class="stat-value stat-value--sm">
              {{ perfOverview.numbers.avg_server_uptime_30d.toFixed(1) }}%
            </p>
          </PpCard>
          <PpCard padding="md" class="stat">
            <p class="stat-label pp-muted">MSPT p95 (30d)</p>
            <p class="stat-value stat-value--sm">{{ perfOverview.numbers.mspt_95th_30d.toFixed(1) }}</p>
          </PpCard>
        </div>
      </PpCard>

      <PpCard v-if="panel === 'performance'" padding="lg" class="section">
        <h3 class="section-title">Players online</h3>
        <p v-if="!planGraph?.series.length" class="pp-muted">
          Online samples are stored from Patch Plan activity exports (plan_tps players_online).
        </p>
        <div v-else class="spark-bars spark-bars--tall" aria-hidden="true">
          <div
            v-for="(pt, idx) in planGraph.series.slice(-96)"
            :key="idx"
            class="spark-bar"
            :style="{
              height: `${Math.max(
                4,
                (pt[1] / Math.max(1, Math.max(...planGraph.series.map((p) => p[1])))) * 100,
              )}%`,
            }"
          />
        </div>
        <h3 class="section-title section-sub">TPS (from server samples)</h3>
        <p v-if="!tpsSpark.length" class="pp-muted">
          TPS samples are recorded every ~5s on servers running Patch Plan export schema v5+.
        </p>
        <div v-else class="spark-bars spark-bars--tall" aria-hidden="true">
          <div
            v-for="(pt, idx) in tpsSpark"
            :key="`t-${idx}`"
            class="spark-bar spark-bar--cpu"
            :style="{ height: `${Math.max(4, pt.v * 100)}%` }"
          />
        </div>
        <h3 class="section-title section-sub">Host CPU & RAM</h3>
        <p v-if="!cpuSpark.length && !ramSpark.length" class="pp-muted">
          Host performance samples are taken when analytics sync runs (sysinfo).
        </p>
        <div v-else class="perf-dual">
          <div>
            <p class="pp-muted perf-label">CPU %</p>
            <div class="spark-bars spark-bars--tall" aria-hidden="true">
              <div
                v-for="(pt, idx) in cpuSpark"
                :key="`c-${idx}`"
                class="spark-bar spark-bar--cpu"
                :style="{ height: `${Math.max(4, pt.v * 100)}%` }"
              />
            </div>
          </div>
          <div>
            <p class="pp-muted perf-label">RAM (bytes)</p>
            <div class="spark-bars spark-bars--tall" aria-hidden="true">
              <div
                v-for="(pt, idx) in ramSpark"
                :key="`r-${idx}`"
                class="spark-bar"
                :style="{ height: `${Math.max(4, pt.v * 100)}%` }"
              />
            </div>
          </div>
        </div>
        <h3 class="section-title section-sub">Free disk (host sync)</h3>
        <p v-if="!(planDiskGraph?.free_disk_space as unknown[])?.length" class="pp-muted">
          Disk samples are written during analytics sync from the desktop host.
        </p>
        <ul
          v-else
          class="mini-stats wide"
        >
          <li
            v-for="(pt, idx) in (planDiskGraph?.free_disk_space as [number, number][]).slice(-8)"
            :key="`d-${idx}`"
          >
            <span>{{ formatEpochMs(pt[0]) }}</span>
            <span class="pp-muted">{{ (pt[1] / 1024).toFixed(0) }} GB free</span>
          </li>
        </ul>
        <h3 class="section-title section-sub">World load (entities / chunks)</h3>
        <p
          v-if="!(planWorldLoad?.entities as unknown[])?.length && !(planWorldLoad?.chunks_loaded as unknown[])?.length"
          class="pp-muted"
        >
          Entity and chunk counts appear when the game reports them in performance exports.
        </p>
        <ul v-else class="mini-stats wide">
          <li
            v-for="(pt, idx) in (planWorldLoad?.entities as [number, number][]).slice(-6)"
            :key="`e-${idx}`"
          >
            <span>{{ formatEpochMs(pt[0]) }}</span>
            <span class="pp-muted">{{ pt[1] }} entities</span>
          </li>
        </ul>
        <h3 class="section-title section-sub">Session punch card</h3>
        <p v-if="!punchGrid" class="pp-muted">Join-time heatmap appears after sessions are ingested.</p>
        <div v-else class="punch-card" aria-label="Sessions by day and hour">
          <div v-for="dow in 7" :key="dow" class="punch-row">
            <div
              v-for="hour in 24"
              :key="hour"
              class="punch-cell"
              :style="{
                opacity: punchGrid.cells.get(`${dow - 1}-${hour - 1}`) ?? 0.08,
              }"
            />
          </div>
        </div>
      </PpCard>

      <PpCard v-if="panel === 'pvp' && planVersus" padding="lg" class="section">
        <h3 class="section-title">PvP & PvE (Plan)</h3>
        <div class="stats-grid stats-grid--compact">
          <PpCard padding="md" class="stat">
            <p class="stat-label pp-muted">Player kills</p>
            <p class="stat-value stat-value--sm">{{ planVersus.numbers.player_kills_total }}</p>
          </PpCard>
          <PpCard padding="md" class="stat">
            <p class="stat-label pp-muted">KDR</p>
            <p class="stat-value stat-value--sm">{{ planVersus.numbers.kdr_total.toFixed(2) }}</p>
          </PpCard>
          <PpCard padding="md" class="stat">
            <p class="stat-label pp-muted">Mob kills (sessions)</p>
            <p class="stat-value stat-value--sm">{{ planVersus.numbers.mob_kills_total }}</p>
          </PpCard>
          <PpCard padding="md" class="stat">
            <p class="stat-label pp-muted">Deaths (sessions)</p>
            <p class="stat-value stat-value--sm">{{ planVersus.numbers.deaths_total }}</p>
          </PpCard>
        </div>
        <p class="pp-muted weapons-line">
          Top weapons:
          {{ planVersus.insights.deadliest_pvp_weapon }},
          {{ planVersus.insights.second_deadliest_pvp_weapon }},
          {{ planVersus.insights.third_deadliest_pvp_weapon }}
        </p>
        <h4 class="section-sub">Recent PvP kills</h4>
        <ul v-if="planKills?.length" class="feed-list">
          <li v-for="(k, i) in planKills" :key="`${k.date}-${i}`" class="feed-row">
            <span class="feed-who">{{ k.killer_name || '?' }}</span>
            <span>→</span>
            <span class="feed-who">{{ k.victim_name || '?' }}</span>
            <span class="pp-muted">{{ k.weapon }}</span>
            <span class="feed-time pp-muted">{{ formatEpochMs(k.date) }}</span>
          </li>
        </ul>
        <p v-else class="pp-muted">No PvP kills in the Plan database yet.</p>
      </PpCard>

      <PpCard
        v-if="planProfile"
        padding="lg"
        class="section profile-card"
      >
        <div class="profile-head">
          <h3 class="section-title">{{ planProfile.info.name }}</h3>
          <PpButton variant="secondary" size="sm" @click="planProfileUuid = null">Close</PpButton>
        </div>
        <p class="pp-muted">
          PAI {{ planProfile.info.activity_index.toFixed(2) }} · {{ planProfile.info.activity_group }}
          · {{ planProfile.info.sessions }} sessions · {{ formatMs(planProfile.info.active_playtime_ms) }} active
        </p>
        <ul class="mini-stats wide profile-meta">
          <li v-if="planProfile.stats">
            <span>Playtime</span>
            <span class="pp-muted">{{ formatMs(planProfile.stats.total_playtime_ms) }} (AFK {{ formatMs(planProfile.stats.afk_time_ms) }})</span>
          </li>
          <li v-if="planProfile.stats?.mob_kills || planProfile.stats?.deaths">
            <span>Combat</span>
            <span class="pp-muted">
              {{ planProfile.stats.mob_kills }} mob · {{ planProfile.stats.deaths }} deaths ·
              {{ planProfile.stats.player_kill_count }} PvP kills
            </span>
          </li>
          <li v-if="planProfile.info.geolocation">
            <span>Region</span>
            <span class="pp-muted">{{ planProfile.info.geolocation }}</span>
          </li>
          <li v-if="planProfile.info.join_address">
            <span>Join address</span>
            <span class="pp-muted mono">{{ planProfile.info.join_address }}</span>
          </li>
          <li v-if="planProfile.info.nick">
            <span>Nick</span>
            <span class="pp-muted">{{ planProfile.info.nick }}</span>
          </li>
          <li v-if="planProfile.info.ping_avg">
            <span>Ping</span>
            <span class="pp-muted mono">
              avg {{ planProfile.info.ping_avg }}ms ({{ planProfile.info.ping_min }}–{{ planProfile.info.ping_max }})
            </span>
          </li>
          <li v-if="planProfile.info.kick_count">
            <span>Kicks</span>
            <span class="pp-muted">{{ planProfile.info.kick_count }}</span>
          </li>
        </ul>
        <h4 v-if="planProfile.nicknames?.length" class="section-sub">Nicknames</h4>
        <ul v-if="planProfile.nicknames?.length" class="mini-stats wide">
          <li v-for="n in planProfile.nicknames.slice(0, 8)" :key="`${n.server_uuid}-${n.nickname}`">
            <span>{{ n.nickname }}</span>
            <span class="pp-muted">{{ n.server_name }} · {{ formatEpochMs(n.last_used) }}</span>
          </li>
        </ul>
        <h4 v-if="planProfile.connections?.length" class="section-sub">Connections</h4>
        <ul v-if="planProfile.connections?.length" class="mini-stats wide">
          <li v-for="c in planProfile.connections" :key="`${c.geolocation}-${c.date}`">
            <span>{{ c.geolocation }}</span>
            <span class="pp-muted">{{ formatEpochMs(c.date) }}</span>
          </li>
        </ul>
        <h4 v-if="planProfile.world_pie_series?.length" class="section-sub">World playtime</h4>
        <ul v-if="planProfile.world_pie_series?.length" class="mini-stats wide">
          <li v-for="w in planProfile.world_pie_series" :key="w.name">
            <span>{{ w.name }}</span>
            <span class="pp-muted">{{ formatMs(w.y) }}</span>
          </li>
        </ul>
        <h4 v-if="planProfile.server_pie_series?.length" class="section-sub">Server playtime</h4>
        <ul v-if="planProfile.server_pie_series?.length" class="mini-stats wide">
          <li v-for="s in planProfile.server_pie_series" :key="s.name">
            <span>{{ s.name }}</span>
            <span class="pp-muted">{{ formatMs(s.y) }}</span>
          </li>
        </ul>
        <h4 class="section-sub">Recent sessions</h4>
        <ul class="feed-list">
          <li v-for="(s, i) in planProfile.sessions.slice(0, 8)" :key="i" class="feed-row">
            <span class="pp-muted">{{ s.server_name }}</span>
            <span>{{ formatEpochMs(s.start) }}</span>
            <span class="pp-muted">{{ formatMs(s.length) }}</span>
          </li>
        </ul>
      </PpCard>

      <PpCard v-if="panel === 'worlds'" padding="lg" class="section">
        <h3 class="section-title">World playtime</h3>
        <ul v-if="worldPie?.world_series?.length" class="mini-stats wide">
          <li v-for="w in (worldPie.world_series as { name: string; y: number }[])" :key="w.name">
            <span>{{ w.name }}</span>
            <span class="pp-muted">{{ formatMs(w.y) }}</span>
          </li>
        </ul>
        <p v-else class="pp-muted">World time is recorded when players change worlds during sessions.</p>
      </PpCard>

      <PpCard v-if="panel === 'geo'" padding="lg" class="section">
        <h3 class="section-title">Geolocation & ping (Plan)</h3>
        <p class="pp-muted">
          Countries are derived from login IPs in-game (private LAN → Local Machine). Ping uses 30-sample
          windows after a 30s join delay.
        </p>
        <h4 class="section-sub">Players by region</h4>
        <ul v-if="(geoGraph as { geolocation_series?: { name: string; value: number }[] })?.geolocation_series?.length" class="mini-stats wide">
          <li
            v-for="g in (geoGraph as { geolocation_series: { name: string; value: number }[] }).geolocation_series"
            :key="g.name"
          >
            <span>{{ g.name }}</span>
            <span class="pp-muted">{{ g.value }} players</span>
          </li>
        </ul>
        <p v-else class="pp-muted">No geolocation data yet — players need to connect with Patch Plan deployed.</p>
        <h4 class="section-sub">Ping table (/v1/pingTable)</h4>
        <ul v-if="pingTable?.length" class="mini-stats wide">
          <li v-for="row in pingTable" :key="row.country">
            <span>{{ row.country }}</span>
            <span class="pp-muted mono">
              avg {{ row.avg_ping }}ms · {{ row.min_ping }}–{{ row.max_ping }}ms
            </span>
          </li>
        </ul>
        <p v-else class="pp-muted">Ping samples appear after players stay online and latency is recorded.</p>
      </PpCard>

      <PpCard v-if="panel === 'query'" padding="lg" class="section">
        <h3 class="section-title">Query builder (Plan /v1/query subset)</h3>
        <p class="pp-muted query-hint">
          Filter players by activity group, join address, and minimum activity index. Results use the local Plan database.
        </p>
        <div v-if="planFilters" class="query-filters">
          <div class="query-group">
            <p class="query-label pp-muted">Activity groups</p>
            <div class="chip-row">
              <button
                v-for="g in planFilters.activity_groups"
                :key="g"
                type="button"
                class="chip"
                :class="{ 'chip--on': queryActivityGroups.includes(g) }"
                @click="toggleQueryGroup(g)"
              >
                {{ g }}
              </button>
            </div>
          </div>
          <div v-if="planFilters.geolocations?.length" class="query-group">
            <p class="query-label pp-muted">Geolocations</p>
            <div class="chip-row">
              <button
                v-for="g in planFilters.geolocations"
                :key="g"
                type="button"
                class="chip"
                :class="{ 'chip--on': queryGeolocations.includes(g) }"
                @click="toggleQueryGeo(g)"
              >
                {{ g }}
              </button>
            </div>
          </div>
          <div v-if="planFilters.join_addresses.length" class="query-group">
            <p class="query-label pp-muted">Join addresses</p>
            <div class="chip-row">
              <button
                v-for="a in planFilters.join_addresses"
                :key="a"
                type="button"
                class="chip"
                :class="{ 'chip--on': queryJoinAddresses.includes(a) }"
                @click="toggleQueryJoin(a)"
              >
                {{ a }}
              </button>
            </div>
          </div>
          <label class="query-min">
            <span class="pp-muted">Min activity index</span>
            <input v-model.number="queryMinActivity" type="number" min="0" max="100" step="0.1" class="query-input" />
          </label>
          <label class="query-min">
            <span class="pp-muted">Min sessions</span>
            <input v-model.number="queryMinSessions" type="number" min="0" step="1" class="query-input" />
          </label>
          <label class="query-min">
            <span class="pp-muted">Registered after</span>
            <input v-model="queryRegisteredAfter" type="date" class="query-input" />
          </label>
          <label class="query-min">
            <span class="pp-muted">Registered before</span>
            <input v-model="queryRegisteredBefore" type="date" class="query-input" />
          </label>
          <label class="query-min">
            <span class="pp-muted">Last seen after</span>
            <input v-model="queryLastSeenAfter" type="date" class="query-input" />
          </label>
          <label class="query-min">
            <span class="pp-muted">Last seen before</span>
            <input v-model="queryLastSeenBefore" type="date" class="query-input" />
          </label>
          <label class="query-min">
            <span class="pp-muted">Cohort view after</span>
            <input v-model="queryViewAfter" type="date" class="query-input" />
          </label>
          <label class="query-min">
            <span class="pp-muted">Cohort view before</span>
            <input v-model="queryViewBefore" type="date" class="query-input" />
          </label>
          <label class="query-min">
            <span class="pp-muted">Played after</span>
            <input v-model="queryPlayedAfter" type="date" class="query-input" />
          </label>
          <label class="query-min">
            <span class="pp-muted">Played before</span>
            <input v-model="queryPlayedBefore" type="date" class="query-input" />
          </label>
          <ul v-if="planFilters?.filter_types?.length" class="filter-type-hint pp-muted">
            <li v-for="f in planFilters.filter_types" :key="f.id">
              {{ f.label }}{{ f.implemented ? '' : ' (planned)' }}
            </li>
          </ul>
          <button
            type="button"
            class="query-run accent-bg"
            :disabled="planQueryLoading"
            @click="runPlanQuery"
          >
            {{ planQueryLoading ? 'Running…' : 'Run query' }}
          </button>
          <label class="query-min query-bypass">
            <input v-model="queryBypassCache" type="checkbox" />
            Bypass 7-day cache
          </label>
        </div>
        <div v-if="planQueryResult" class="query-results">
          <p class="pp-muted">
            Matched {{ planQueryResult.matched_count }} players · avg activity
            {{ planQueryResult.avg_activity_index.toFixed(1) }} · avg active playtime
            {{ formatMs(planQueryResult.avg_active_playtime_ms) }}
            <span v-if="planQueryResult.from_cache" class="query-cache-badge">cached</span>
          </p>
          <p v-if="planQueryResult.cache_token" class="query-share pp-muted">
            Share token:
            <code class="mono">{{ planQueryResult.cache_token }}</code>
            <button type="button" class="query-share-btn" @click="copyQueryShareLink">
              {{ queryShareCopied ? 'Copied' : 'Copy link' }}
            </button>
          </p>
          <div v-if="planQueryResult.cohort_view" class="cohort-mini stats-grid">
            <div>
              <p class="pp-muted">Cohort sessions</p>
              <p>{{ planQueryResult.cohort_view.sessions_in_window }}</p>
            </div>
            <div>
              <p class="pp-muted">Playtime in window</p>
              <p>{{ formatMs(planQueryResult.cohort_view.playtime_ms) }}</p>
            </div>
            <div>
              <p class="pp-muted">AFK in window</p>
              <p>{{ formatMs(planQueryResult.cohort_view.afk_ms) }}</p>
            </div>
            <div>
              <p class="pp-muted">Active in window</p>
              <p>{{ planQueryResult.cohort_view.players_active_in_window }} players</p>
            </div>
          </div>
          <ul v-if="planQueryResult.players.length" class="plan-table">
            <li v-for="p in planQueryResult.players.slice(0, 80)" :key="p.uuid" class="plan-row">
              <button type="button" class="plan-name" @click="planProfileUuid = p.uuid">{{ p.name }}</button>
              <span class="pp-muted">{{ p.activity_group }}</span>
              <span>{{ p.activity_index.toFixed(1) }}</span>
              <span class="pp-muted">{{ p.join_address || '—' }}</span>
            </li>
          </ul>
        </div>
        <h3 class="section-title section-sub">Retention (new players, 30d)</h3>
        <ul v-if="planRetention?.new_players.length" class="plan-table">
          <li v-for="p in planRetention.new_players.slice(0, 40)" :key="p.uuid" class="plan-row">
            <button type="button" class="plan-name" @click="planProfileUuid = p.uuid">{{ p.name }}</button>
            <span>{{ p.activity_index.toFixed(1) }}</span>
            <span class="pp-muted">{{ p.session_count }} sessions</span>
            <span class="pp-muted">{{ formatMs(p.active_playtime_ms) }}</span>
          </li>
        </ul>
        <p v-else class="pp-muted">New-player retention rows appear after sessions sync into the Plan database.</p>
        <h3 class="section-title section-sub">Join addresses</h3>
        <ul v-if="joinAddresses?.length" class="mini-stats wide">
          <li v-for="j in joinAddresses" :key="j.join_address">
            <span>{{ j.join_address }}</span>
            <span class="pp-muted">{{ j.count }} sessions</span>
          </li>
        </ul>
        <p v-else class="pp-muted">Join addresses appear once virtual host data is captured on login.</p>
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
.install-banner {
  margin: 0 0 1rem;
  padding: 0.65rem 0.85rem;
  border-radius: var(--radius-md);
  border: 1px solid rgba(230, 126, 34, 0.35);
  background: var(--color-accent-muted);
  font-size: 0.85rem;
  color: var(--color-text);
}
.plan-register-card {
  margin-bottom: 1rem;
}
.plan-register-form {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  align-items: flex-end;
  margin-top: 0.75rem;
}
.plan-register-form label {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  font-size: 0.85rem;
}
.plan-register-form input {
  min-width: 10rem;
  padding: 0.35rem 0.5rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg-elevated);
  color: inherit;
}
.plan-users-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.85rem;
  margin-top: 0.5rem;
}
.plan-users-table th,
.plan-users-table td {
  padding: 0.35rem 0.5rem;
  border-bottom: 1px solid var(--color-border-subtle);
  text-align: left;
}
.plan-web-users {
  margin-top: 1rem;
}

.plan-register-code {
  margin-top: 0.75rem;
  padding: 0.75rem;
  border-radius: var(--radius-md);
  background: var(--color-bg-elevated);
}

.install-banner--error {
  border-color: rgba(231, 76, 60, 0.45);
  background: rgba(231, 76, 60, 0.12);
}
.export-warning {
  margin-bottom: 1rem;
  border-color: rgba(241, 196, 15, 0.45);
}
.export-warning__title {
  margin: 0 0 0.35rem;
  font-size: 0.95rem;
}
.export-warning__lead {
  margin: 0 0 0.65rem;
  font-size: 0.85rem;
}
.export-warning__list {
  margin: 0;
  padding-left: 1.15rem;
  font-size: 0.85rem;
}
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
.stats-grid--compact {
  margin-bottom: 0.75rem;
}
.section-sub {
  margin: 0.75rem 0 0.35rem;
  font-size: 0.9rem;
}
.weapons-line {
  font-size: 0.85rem;
  margin: 0 0 0.5rem;
}
.plan-link {
  border: none;
  background: none;
  padding: 0;
  cursor: pointer;
  color: var(--color-accent-hover);
  font-weight: 600;
}
.profile-card {
  margin-top: 1rem;
  border-color: rgba(230, 126, 34, 0.35);
}
.profile-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.5rem;
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
.plan-table {
  list-style: none;
  margin: 0 0 1.25rem;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.plan-table__row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
  font-size: 0.85rem;
  padding-bottom: 0.35rem;
  border-bottom: 1px solid var(--color-border-subtle);
}
.section-title--sub {
  margin-top: 1rem;
  font-size: 0.9rem;
}
.spark-bars {
  display: flex;
  align-items: flex-end;
  gap: 2px;
  height: 4.5rem;
}
.spark-bars--tall {
  height: 6rem;
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
.player-meta {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 0.2rem;
}
.activity-badge {
  font-size: 0.65rem;
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
.perf-dual {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;
  margin-bottom: 1rem;
}
.perf-label {
  margin: 0 0 0.35rem;
  font-size: 0.75rem;
}
.spark-bar--cpu {
  background: rgba(52, 152, 219, 0.85);
}
.punch-card {
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-width: 28rem;
}
.punch-row {
  display: grid;
  grid-template-columns: repeat(24, 1fr);
  gap: 2px;
}
.punch-cell {
  aspect-ratio: 1;
  border-radius: 2px;
  background: var(--color-accent-hover);
}
.query-hint {
  margin: 0 0 0.75rem;
  font-size: 0.85rem;
}
.query-filters {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  margin-bottom: 1rem;
}
.chip-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
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
.query-min {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  max-width: 10rem;
}
.query-input {
  padding: 0.4rem 0.5rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg-elevated);
  color: inherit;
}
.query-run {
  align-self: flex-start;
  padding: 0.45rem 1rem;
  border: none;
  border-radius: var(--radius-md);
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}
.query-bypass {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.query-share {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
  margin-top: 0.5rem;
}

.query-share-btn {
  padding: 0.25rem 0.75rem;
  border-radius: 0.5rem;
  border: 1px solid var(--pp-border, rgba(255, 255, 255, 0.12));
  background: transparent;
  cursor: pointer;
  transition: all 0.2s;
}

.query-cache-badge {
  margin-left: 0.5rem;
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.query-run:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
.filter-type-hint {
  list-style: none;
  margin: 0;
  padding: 0;
  font-size: 0.75rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem 1rem;
}
</style>
