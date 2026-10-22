<script setup lang="ts">
import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { ArrowDownTrayIcon, MagnifyingGlassIcon, XMarkIcon } from '@heroicons/vue/24/outline'
import type { PumpkinMarketPluginSummary, ServerRecord } from '~/types/patch'

const props = defineProps<{
  server: ServerRecord
}>()

const api = usePatchApi()
const queryClient = useQueryClient()
const query = ref('')
const debouncedQuery = ref('')
const category = ref('all')
const selectedId = ref<number | null>(null)
let debounceTimer: ReturnType<typeof setTimeout> | null = null

const categoryOptions = [
  { value: 'all', label: 'All categories' },
  { value: 'Utilities', label: 'Utilities' },
  { value: 'World Management', label: 'World Management' },
  { value: 'Admin Tools', label: 'Admin Tools' },
  { value: 'Fun', label: 'Fun' },
  { value: 'Other', label: 'Other' },
]

watch(query, (q) => {
  if (debounceTimer) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => {
    debouncedQuery.value = q
  }, 350)
})

const {
  data: marketPages,
  isFetching,
  isFetchingNextPage,
  fetchNextPage,
  hasNextPage,
} = useInfiniteQuery({
  queryKey: ['pumpkin-market', debouncedQuery, category],
  queryFn: ({ pageParam }) =>
    api.pumpkinMarketList(debouncedQuery.value, category.value, 12, pageParam ?? undefined),
  initialPageParam: null as string | null,
  getNextPageParam: (last) => last.next_cursor,
})

const items = computed(
  () => marketPages.value?.pages.flatMap((page) => page.items) ?? [],
)

const { data: detail, isFetching: detailLoading } = useQuery({
  queryKey: ['pumpkin-market-detail', selectedId],
  queryFn: () => api.pumpkinMarketGet(selectedId.value!),
  enabled: computed(() => selectedId.value != null),
})

const { data: installed, refetch: refetchInstalled } = useQuery({
  queryKey: ['server-plugins', props.server.id],
  queryFn: () => api.serverListPlugins(props.server.id),
})

const installingId = ref<number | null>(null)
const installError = ref<string | null>(null)

const installMutation = useMutation({
  mutationFn: async (pluginId: number) => {
    installingId.value = pluginId
    installError.value = null
    return api.pumpkinMarketInstall(pluginId, props.server.data_path)
  },
  onSuccess: () => {
    refetchInstalled()
    queryClient.invalidateQueries({ queryKey: ['server-plugins', props.server.id] })
  },
  onError: (err: Error) => {
    installError.value = err.message || 'Install failed'
  },
  onSettled: () => {
    installingId.value = null
  },
})

const removeMutation = useMutation({
  mutationFn: (filename: string) => api.serverRemovePlugin(props.server.id, filename),
  onSuccess: () => refetchInstalled(),
})

function openDetail(hit: PumpkinMarketPluginSummary) {
  selectedId.value = hit.id
}

function closeDetail() {
  selectedId.value = null
}

function shortDescription(text: string, max = 160) {
  const plain = text.replace(/[#>*`\[\]]/g, ' ').replace(/\s+/g, ' ').trim()
  if (plain.length <= max) return plain
  return `${plain.slice(0, max)}…`
}

function priceLabel(cents: number, type: string) {
  if (type === 'free' || cents === 0) return 'Free'
  return `$${(cents / 100).toFixed(2)}`
}

function formatBytes(n: number) {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / (1024 * 1024)).toFixed(1)} MB`
}

function canInstall(hit: { type: string; price_cents: number }) {
  return hit.type === 'free' || hit.price_cents === 0
}

function installFromDetail() {
  if (!detail.value || !canInstall(detail.value)) return
  installMutation.mutate(detail.value.id)
}
</script>

<template>
  <div class="browser">
    <PpCard padding="lg" class="market-intro">
      <p class="pp-muted intro-text">
        Official Pumpkin plugins from
        <a href="https://market.pumpkinmc.org/" target="_blank" rel="noopener" class="link">
          market.pumpkinmc.org
        </a>
        . Search uses the same <code class="pp-code">q</code> API as the website; free plugins
        install into <code class="pp-code">plugins/</code> on this server.
      </p>
    </PpCard>

    <PpCard padding="lg">
      <h3 class="section-title">Installed on {{ server.name }}</h3>
      <p class="pp-muted restart-hint">Restart the network after installing or removing plugins.</p>
      <p v-if="!installed?.length" class="pp-muted">No plugins yet. Browse the market below.</p>
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

    <div class="filters" data-tauri-drag-region-exclude>
      <MagnifyingGlassIcon class="search-ico" aria-hidden="true" />
      <PpInput v-model="query" label="Search" placeholder="EpicRTP, void, economy…" class="search-field" />
      <PpSelect v-model="category" label="Category" :options="categoryOptions" class="category-field" />
    </div>

    <p v-if="installError" class="install-error" role="alert">{{ installError }}</p>
    <p v-if="isFetching && !items.length" class="pp-muted">Loading market…</p>
    <p v-else-if="!isFetching && !items.length" class="pp-muted">No plugins match your search.</p>

    <div v-if="items.length" class="hits">
      <article v-for="hit in items" :key="hit.id" class="hit">
        <button type="button" class="hit-open" @click="openDetail(hit)">
          <img
            v-if="hit.preview_path"
            :src="hit.preview_path"
            :alt="hit.name"
            class="hit-icon"
            loading="lazy"
          />
          <div v-else class="hit-icon hit-icon--placeholder" aria-hidden="true">🎃</div>
          <div class="hit-body">
            <div class="hit-head">
              <h4 class="hit-title">{{ hit.name }}</h4>
              <PpBadge tone="neutral">{{ hit.category }}</PpBadge>
              <PpBadge v-if="hit.is_early_access" tone="warning">Early access</PpBadge>
              <span class="pp-muted hit-meta">{{ priceLabel(hit.price_cents, hit.type) }}</span>
            </div>
            <p class="hit-desc pp-muted">{{ shortDescription(hit.description) }}</p>
            <p class="hit-foot pp-muted">
              {{ hit.dev_name }} · v{{ hit.version || '?' }} ·
              {{ hit.downloads.toLocaleString() }} downloads
            </p>
          </div>
        </button>
        <PpButton
          v-if="canInstall(hit)"
          size="sm"
          :loading="installingId === hit.id"
          :disabled="installMutation.isPending.value && installingId !== hit.id"
          @click="installMutation.mutate(hit.id)"
        >
          <ArrowDownTrayIcon class="btn-ico" aria-hidden="true" />
          Install
        </PpButton>
        <PpButton v-else size="sm" variant="secondary" disabled title="Purchase on market first">
          Purchase required
        </PpButton>
      </article>
    </div>

    <div v-if="hasNextPage" class="more-row">
      <PpButton variant="secondary" :loading="isFetchingNextPage" @click="fetchNextPage()">
        Load more
      </PpButton>
    </div>

    <div v-if="selectedId != null" class="detail-backdrop" @click.self="closeDetail">
      <div class="detail-panel" role="dialog" aria-modal="true" aria-labelledby="market-detail-title">
        <header class="detail-header">
          <h2 id="market-detail-title">{{ detail?.name ?? 'Plugin' }}</h2>
          <PpIconButton label="Close" @click="closeDetail">
            <XMarkIcon aria-hidden="true" />
          </PpIconButton>
        </header>
        <p v-if="detailLoading" class="pp-muted detail-body">Loading details…</p>
        <div v-else-if="detail" class="detail-body">
          <div class="detail-meta">
            <PpBadge tone="neutral">{{ detail.category }}</PpBadge>
            <span class="pp-muted">{{ detail.dev_name }}</span>
            <span class="pp-muted">v{{ detail.version }}</span>
            <span class="pp-muted">{{ priceLabel(detail.price_cents, detail.type) }}</span>
            <span v-if="detail.file_size" class="pp-muted">{{ formatBytes(detail.file_size) }}</span>
          </div>
          <p v-if="detail.keywords" class="pp-muted keywords">Keywords: {{ detail.keywords }}</p>
          <pre class="detail-desc">{{ detail.description }}</pre>
          <div v-if="detail.screenshots.length" class="screenshots">
            <img
              v-for="(url, i) in detail.screenshots"
              :key="i"
              :src="url"
              alt=""
              class="screenshot"
              loading="lazy"
            />
          </div>
          <section v-if="detail.commands.length" class="detail-section">
            <h3>Commands</h3>
            <ul class="cmd-list">
              <li v-for="(cmd, i) in detail.commands" :key="i">
                <code class="pp-code">{{ cmd.name }}</code>
                <span class="pp-muted">{{ cmd.description }}</span>
              </li>
            </ul>
          </section>
          <section v-if="detail.versions.length" class="detail-section">
            <h3>Versions</h3>
            <ul class="version-list">
              <li v-for="v in detail.versions" :key="v.id">
                <strong>{{ v.version }}</strong>
                <span v-if="v.track" class="pp-muted">({{ v.track }})</span>
                <p v-if="v.release_notes" class="pp-muted version-notes">{{ v.release_notes }}</p>
              </li>
            </ul>
          </section>
          <div class="detail-actions">
            <a
              :href="`https://market.pumpkinmc.org/plugin/${detail.id}`"
              target="_blank"
              rel="noopener"
              class="link"
            >
              Open on market.pumpkinmc.org
            </a>
            <PpButton
              v-if="canInstall(detail)"
              :loading="installingId === detail.id"
              @click="installFromDetail"
            >
              <ArrowDownTrayIcon class="btn-ico" aria-hidden="true" />
              Install to {{ server.name }}
            </PpButton>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.browser {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}
.intro-text {
  margin: 0;
  font-size: 0.9rem;
  line-height: 1.5;
}
.link {
  color: var(--color-accent-hover);
  text-decoration: none;
}
.link:hover {
  text-decoration: underline;
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
  justify-content: space-between;
  gap: 0.75rem;
  flex-wrap: wrap;
  font-size: 0.85rem;
}
.installed-name {
  font-family: ui-monospace, monospace;
  flex: 1;
}
.filters {
  display: flex;
  align-items: flex-end;
  gap: 0.75rem;
  flex-wrap: wrap;
}
.search-field {
  flex: 1;
  min-width: 12rem;
}
.category-field {
  min-width: 11rem;
}
.search-ico {
  width: 1.1rem;
  height: 1.1rem;
  margin-bottom: 0.55rem;
  color: var(--color-text-muted);
}
.install-error {
  margin: 0;
  font-size: 0.85rem;
  color: var(--color-danger, #e74c3c);
}
.hits {
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
}
.hit {
  display: flex;
  gap: 0.65rem;
  align-items: flex-start;
  padding: 0.85rem 1rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg-elevated);
}
.hit-open {
  display: flex;
  gap: 0.85rem;
  flex: 1;
  min-width: 0;
  padding: 0;
  border: none;
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}
.hit-open:hover .hit-title {
  color: var(--color-accent-hover);
}
.hit-icon {
  width: 3rem;
  height: 3rem;
  border-radius: var(--radius-md);
  object-fit: cover;
  flex-shrink: 0;
}
.hit-icon--placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.5rem;
  background: var(--color-bg);
}
.hit-body {
  flex: 1;
  min-width: 0;
}
.hit-head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.4rem;
  margin-bottom: 0.35rem;
}
.hit-title {
  margin: 0;
  font-size: 0.95rem;
  transition: color 0.2s ease;
}
.hit-meta {
  font-size: 0.8rem;
}
.hit-desc {
  margin: 0 0 0.35rem;
  font-size: 0.85rem;
  line-height: 1.45;
}
.hit-foot {
  margin: 0;
  font-size: 0.75rem;
}
.btn-ico {
  width: 1rem;
  height: 1rem;
}
.more-row {
  display: flex;
  justify-content: center;
  padding: 0.5rem 0 1rem;
}
.detail-backdrop {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(4px);
}
.detail-panel {
  width: min(42rem, 100%);
  max-height: min(85vh, 40rem);
  overflow: auto;
  border-radius: var(--radius-lg);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg-elevated);
  padding: 1.25rem 1.5rem;
}
.detail-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  margin-bottom: 1rem;
}
.detail-header h2 {
  margin: 0;
  font-size: 1.15rem;
}
.detail-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  align-items: center;
  margin-bottom: 0.75rem;
  font-size: 0.85rem;
}
.keywords {
  margin: 0 0 0.75rem;
  font-size: 0.8rem;
}
.detail-desc {
  margin: 0 0 1rem;
  white-space: pre-wrap;
  font-family: inherit;
  font-size: 0.85rem;
  line-height: 1.55;
  color: var(--color-text-muted);
}
.screenshots {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
  margin-bottom: 1rem;
}
.screenshot {
  max-width: 100%;
  max-height: 10rem;
  border-radius: var(--radius-md);
  object-fit: cover;
}
.detail-section h3 {
  margin: 0 0 0.5rem;
  font-size: 0.9rem;
}
.cmd-list,
.version-list {
  margin: 0 0 1rem;
  padding-left: 1.1rem;
  font-size: 0.85rem;
}
.cmd-list li,
.version-list li {
  margin-bottom: 0.35rem;
}
.version-notes {
  margin: 0.25rem 0 0;
  font-size: 0.8rem;
}
.detail-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  align-items: center;
  padding-top: 0.5rem;
  border-top: 1px solid var(--color-border-subtle);
}
</style>
