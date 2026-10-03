<script setup lang="ts">
import { useQuery } from '@tanstack/vue-query'

const route = useRoute()
const api = usePatchApi()
const id = computed(() => route.params.id as string)

const { data: network, isLoading } = useQuery({
  queryKey: ['network', id],
  queryFn: () => api.networkGet(id.value),
})

const targetServerId = ref('')

watch(
  network,
  (n) => {
    if (!n) return
    if (!targetServerId.value) {
      targetServerId.value =
        n.servers.find((s) => s.role === 'lobby')?.id ?? n.servers[0]?.id ?? null
    }
  },
  { immediate: true },
)

const pluginServer = computed(() =>
  network.value?.servers.find((s) => s.id === targetServerId.value),
)

type CatalogTab = 'market' | 'modrinth'
const catalogTab = ref<CatalogTab>('market')
</script>

<template>
  <div class="pp-page">
    <NetworkSubnav />
    <PpPageHeader
      title="Plugins"
      description="Install Pumpkin Market and Modrinth WASM plugins onto any server instance."
    />
    <p v-if="isLoading" class="pp-muted">Loading…</p>
    <template v-else-if="network && pluginServer">
      <PpSelect
        v-if="network.servers.length > 1"
        v-model="targetServerId"
        label="Target server"
        :options="
          network.servers.map((s) => ({
            value: s.id,
            label: `${s.name} (${s.role})`,
          }))
        "
        class="server-picker"
      />
      <div class="catalog-tabs" data-tauri-drag-region-exclude>
        <button
          type="button"
          class="catalog-tab"
          :class="{ 'catalog-tab--active': catalogTab === 'market' }"
          @click="catalogTab = 'market'"
        >
          Pumpkin Market
        </button>
        <button
          type="button"
          class="catalog-tab"
          :class="{ 'catalog-tab--active': catalogTab === 'modrinth' }"
          @click="catalogTab = 'modrinth'"
        >
          Modrinth
        </button>
      </div>
      <PumpkinMarketBrowser
        v-if="catalogTab === 'market'"
        :key="`${pluginServer.id}-market`"
        :server="pluginServer"
      />
      <ModrinthPluginBrowser
        v-else
        :key="`${pluginServer.id}-modrinth`"
        :server="pluginServer"
      />
    </template>
  </div>
</template>

<style scoped>
.server-picker {
  max-width: 20rem;
  margin-bottom: 1rem;
}
.catalog-tabs {
  display: flex;
  gap: 0.35rem;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}
.catalog-tab {
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
.catalog-tab:hover {
  color: var(--color-text);
  background: var(--color-surface-hover);
}
.catalog-tab--active {
  color: var(--color-accent-hover);
  border-color: rgba(230, 126, 34, 0.45);
  background: var(--color-accent-muted);
}
</style>
