<script setup lang="ts">
import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
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

const {
  data: marketPages,
  isFetching,
  isFetchingNextPage,
  fetchNextPage,
  hasNextPage,
} = useInfiniteQuery({
  queryKey: ['pumpkin-market', debouncedQuery],
  queryFn: ({ pageParam }) =>
    api.pumpkinMarketList(debouncedQuery.value, 12, pageParam ?? undefined),
  initialPageParam: null as string | null,
  getNextPageParam: (last) => last.next_cursor,
})

const items = computed(
  () => marketPages.value?.pages.flatMap((page) => page.items) ?? [],
)

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
</script>

<template>
  <div class="browser">
    <PpCard padding="lg" class="market-intro">
      <p class="pp-muted intro-text">
        Browse official Pumpkin plugins from
        <a href="https://market.pumpkinmc.org/" target="_blank" rel="noopener" class="link">
          market.pumpkinmc.org
        </a>
        . Free plugins install directly into this server&apos;s <code class="pp-code">plugins/</code>
        folder.
      </p>
    </PpCard>

    <PpCard padding="lg">
      <h3 class="section-title">Installed on {{ server.name }}</h3>
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

    <div class="search-row">
      <MagnifyingGlassIcon class="search-ico" aria-hidden="true" />
      <PpInput v-model="query" label="Search Pumpkin Market" placeholder="TPS, void, economy…" />
    </div>

    <p v-if="installError" class="install-error" role="alert">{{ installError }}</p>

    <p v-if="isFetching && !items.length" class="pp-muted">Loading market…</p>

    <div v-if="items.length" class="hits">
      <article v-for="hit in items" :key="hit.id" class="hit">
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
        <PpButton
          v-else
          size="sm"
          variant="secondary"
          disabled
          title="Purchase on market.pumpkinmc.org first"
        >
          Purchase required
        </PpButton>
      </article>
    </div>

    <div v-if="hasNextPage" class="more-row">
      <PpButton
        variant="secondary"
        :loading="isFetchingNextPage"
        @click="fetchNextPage()"
      >
        Load more
      </PpButton>
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
.search-row {
  display: flex;
  align-items: flex-end;
  gap: 0.5rem;
}
.search-row .pp-field {
  flex: 1;
}
.search-ico {
  width: 1.1rem;
  height: 1.1rem;
  margin-bottom: 0.55rem;
  color: var(--color-text-muted);
  flex-shrink: 0;
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
  gap: 0.85rem;
  align-items: flex-start;
  padding: 0.85rem 1rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg-elevated);
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
</style>
