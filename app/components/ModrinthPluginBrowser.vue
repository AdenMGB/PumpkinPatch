<script setup lang="ts">
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { ArrowDownTrayIcon, MagnifyingGlassIcon } from '@heroicons/vue/24/outline'
import type { ServerRecord } from '~/types/patch'

const props = defineProps<{
  server: ServerRecord
}>()

const api = usePatchApi()
const queryClient = useQueryClient()
const query = ref('')
const debouncedQuery = ref('')
let debounceTimer: ReturnType<typeof setTimeout> | null = null

watch(query, (q) => {
  if (debounceTimer) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => {
    debouncedQuery.value = q
  }, 350)
})

const { data: search, isFetching } = useQuery({
  queryKey: ['modrinth', debouncedQuery, props.server.minecraft_version],
  queryFn: () =>
    api.modrinthSearch(debouncedQuery.value, props.server.minecraft_version, 24, 0),
})

const { data: installed, refetch: refetchInstalled } = useQuery({
  queryKey: ['server-plugins', props.server.id],
  queryFn: () => api.serverListPlugins(props.server.id),
})

const installMutation = useMutation({
  mutationFn: async (hit: { project_id: string; title: string }) => {
    const versions = await api.modrinthProjectVersions(
      hit.project_id,
      props.server.minecraft_version,
    )
    const version = versions[0]
    if (!version) throw new Error('No compatible version')
    return api.modrinthInstall(hit.project_id, version.id, props.server.data_path)
  },
  onSuccess: () => {
    refetchInstalled()
    queryClient.invalidateQueries({ queryKey: ['server-plugins', props.server.id] })
  },
})

const removeMutation = useMutation({
  mutationFn: (filename: string) => api.serverRemovePlugin(props.server.id, filename),
  onSuccess: () => refetchInstalled(),
})

function formatBytes(n: number) {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / (1024 * 1024)).toFixed(1)} MB`
}
</script>

<template>
  <div class="browser">
    <PpCard padding="lg">
      <h3 class="section-title">Installed on {{ server.name }}</h3>
      <p v-if="!installed?.length" class="pp-muted">No plugins yet. Browse Modrinth below.</p>
      <ul v-else class="installed-list">
        <li v-for="p in installed" :key="p.path" class="installed-row">
          <span class="installed-name">{{ p.filename }}</span>
          <span class="pp-muted">{{ formatBytes(p.size_bytes) }}</span>
          <PpButton
            size="sm"
            variant="danger"
            :loading="removeMutation.isPending.value"
            @click="removeMutation.mutate(p.filename)"
          >
            Remove
          </PpButton>
        </li>
      </ul>
    </PpCard>

    <div class="search-row">
      <div class="search-wrap">
        <MagnifyingGlassIcon class="search-icon" aria-hidden="true" />
        <input
          v-model="query"
          type="search"
          class="search-input"
          placeholder="Search Modrinth plugins…"
          data-tauri-drag-region-exclude
        />
      </div>
      <PpBadge tone="accent">MC {{ server.minecraft_version }}</PpBadge>
    </div>

    <p v-if="isFetching" class="pp-muted">Searching…</p>

    <div class="hits-grid">
      <PpCard
        v-for="hit in search?.hits ?? []"
        :key="hit.project_id"
        padding="md"
        hover
        class="hit-card"
      >
        <div class="hit-top">
          <img v-if="hit.icon_url" :src="hit.icon_url" alt="" class="hit-icon" />
          <div v-else class="hit-icon hit-icon--placeholder">?</div>
          <div>
            <h4>{{ hit.title }}</h4>
            <p class="pp-muted hit-author">by {{ hit.author }}</p>
          </div>
        </div>
        <p class="hit-desc">{{ hit.description }}</p>
        <div class="hit-footer">
          <span class="pp-muted">{{ hit.downloads.toLocaleString() }} downloads</span>
          <PpButton
            size="sm"
            :loading="installMutation.isPending.value"
            @click="installMutation.mutate(hit)"
          >
            <ArrowDownTrayIcon class="btn-icon" aria-hidden="true" />
            Install
          </PpButton>
        </div>
      </PpCard>
    </div>
  </div>
</template>

<style scoped>
.browser {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}
.section-title {
  margin: 0 0 0.75rem;
  font-size: 1rem;
}
.installed-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.installed-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
}
.installed-name {
  font-family: ui-monospace, monospace;
  font-size: 0.85rem;
  flex: 1;
}
.search-row {
  display: flex;
  gap: 0.75rem;
  align-items: center;
  flex-wrap: wrap;
}
.search-wrap {
  flex: 1;
  min-width: 14rem;
  position: relative;
}
.search-icon {
  position: absolute;
  left: 0.75rem;
  top: 50%;
  transform: translateY(-50%);
  width: 1.1rem;
  height: 1.1rem;
  color: var(--color-text-muted);
}
.search-input {
  width: 100%;
  padding: 0.6rem 0.75rem 0.6rem 2.35rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg-elevated);
  color: var(--color-text);
  font-size: 0.9rem;
}
.search-input:focus {
  outline: none;
  border-color: var(--color-accent);
  box-shadow: 0 0 0 3px var(--color-accent-muted);
}
.hits-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(17rem, 1fr));
  gap: 1rem;
}
.hit-top {
  display: flex;
  gap: 0.75rem;
  align-items: flex-start;
  margin-bottom: 0.5rem;
}
.hit-icon {
  width: 2.5rem;
  height: 2.5rem;
  border-radius: var(--radius-md);
  object-fit: cover;
}
.hit-icon--placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-surface-hover);
  color: var(--color-text-muted);
}
h4 {
  margin: 0;
  font-size: 0.95rem;
}
.hit-author {
  margin: 0.15rem 0 0;
  font-size: 0.75rem;
}
.hit-desc {
  margin: 0 0 0.75rem;
  font-size: 0.8rem;
  line-height: 1.45;
  color: var(--color-text-muted);
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.hit-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.75rem;
}
.btn-icon {
  width: 1rem;
  height: 1rem;
}
</style>
