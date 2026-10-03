<script setup lang="ts">
import { useQuery } from '@tanstack/vue-query'
import { Cog6ToothIcon } from '@heroicons/vue/24/outline'

const route = useRoute()
const api = usePatchApi()
const id = computed(() => route.params.id as string)

const { data: network, isLoading } = useQuery({
  queryKey: ['network', id],
  queryFn: () => api.networkGet(id.value),
})

type SettingsTab = 'hub' | 'server'
const tab = ref<SettingsTab>('hub')
const selectedServerId = ref('')

watch(
  [network, () => route.query.server],
  ([n, serverQuery]) => {
    if (!n) return
    if (typeof serverQuery === 'string' && serverQuery) {
      tab.value = 'server'
      selectedServerId.value = serverQuery
      return
    }
    if (!selectedServerId.value) {
      selectedServerId.value =
        n.servers.find((s) => s.role === 'lobby')?.id ?? n.servers[0]?.id ?? ''
    }
  },
  { immediate: true },
)

const selectedServer = computed(() =>
  network.value?.servers.find((s) => s.id === selectedServerId.value),
)

function selectServer(serverId: string) {
  tab.value = 'server'
  selectedServerId.value = serverId
}
</script>

<template>
  <div class="pp-page settings-page">
    <NetworkSubnav />
    <PpPageHeader
      title="Network settings"
      description="Hub proxy, Pumpkin instances, versions, and gameplay — separate from the overview."
    />

    <p v-if="isLoading" class="pp-muted">Loading…</p>
    <template v-else-if="network">
      <div class="settings-layout">
        <aside class="settings-nav">
          <button
            type="button"
            class="nav-item"
            :class="{ 'nav-item--active': tab === 'hub' }"
            @click="tab = 'hub'"
          >
            <Cog6ToothIcon class="nav-icon" aria-hidden="true" />
            Velocity hub
          </button>
          <p class="nav-heading">Pumpkin servers</p>
          <button
            v-for="s in network.servers"
            :key="s.id"
            type="button"
            class="nav-item"
            :class="{ 'nav-item--active': tab === 'server' && selectedServerId === s.id }"
            @click="selectServer(s.id)"
          >
            <span class="nav-server-name">{{ s.name }}</span>
            <PpBadge tone="neutral">{{ s.role }}</PpBadge>
          </button>
        </aside>

        <div class="settings-content">
          <HubSettingsPanel v-if="tab === 'hub'" :network-id="network.network.id" />
          <ServerSettingsPanel
            v-else-if="selectedServer"
            :key="selectedServer.id"
            :server="selectedServer"
            full-page
          />
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.settings-page {
  padding-bottom: 2rem;
}
.settings-layout {
  display: grid;
  grid-template-columns: minmax(12rem, 16rem) minmax(0, 1fr);
  gap: 1.25rem;
  align-items: start;
}
@media (max-width: 800px) {
  .settings-layout {
    grid-template-columns: 1fr;
  }
}
.settings-nav {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  position: sticky;
  top: 0;
}
.nav-heading {
  margin: 0.75rem 0 0.25rem;
  padding: 0 0.5rem;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--color-text-muted);
  font-weight: 700;
}
.nav-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: 100%;
  padding: 0.55rem 0.65rem;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--color-text-muted);
  font-size: 0.875rem;
  font-weight: 600;
  cursor: pointer;
  text-align: left;
  transition: all 0.2s ease;
}
.nav-item:hover {
  background: var(--color-surface-hover);
  color: var(--color-text);
}
.nav-item--active {
  background: var(--color-accent-muted);
  border-color: rgba(230, 126, 34, 0.35);
  color: var(--color-accent-hover);
}
.nav-icon {
  width: 1.1rem;
  height: 1.1rem;
  flex-shrink: 0;
}
.nav-server-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}
.settings-content {
  min-width: 0;
}
</style>
