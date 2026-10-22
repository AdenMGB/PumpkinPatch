<script setup lang="ts">
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import type { ServerRecord } from '~/types/patch'

const props = defineProps<{
  server: ServerRecord
}>()

const api = usePatchApi()
const queryClient = useQueryClient()

const { data: manifest } = useQuery({
  queryKey: ['bundled-plugins'],
  queryFn: () => api.pluginListAvailable(),
})

const deployMutation = useMutation({
  mutationFn: (pluginId: string) =>
    api.pluginDeploy(pluginId, props.server.data_path, props.server.role),
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['server-plugins', props.server.id] })
  },
})

const entries = computed(() =>
  (manifest.value ?? []).filter(
    (e) => !e.targets.length || e.targets.includes(props.server.role),
  ),
)
</script>

<template>
  <PpCard padding="lg" class="bundled">
    <h3 class="section-title">Bundled manifest plugins</h3>
    <p class="pp-muted desc">
      Ship with Pumpkin Patch (<code class="pp-code">plugins/manifest.json</code>). Restart the
      network after deploying so Pumpkin loads new WASM files.
    </p>
    <ul v-if="entries.length" class="list">
      <li v-for="entry in entries" :key="entry.id" class="row">
        <div>
          <span class="name">{{ entry.name }}</span>
          <span class="pp-muted meta">v{{ entry.version }} · {{ entry.id }}</span>
          <p class="pp-muted blurb">{{ entry.description }}</p>
        </div>
        <PpButton
          size="sm"
          variant="secondary"
          :loading="deployMutation.isPending.value"
          @click="deployMutation.mutate(entry.id)"
        >
          Deploy
        </PpButton>
      </li>
    </ul>
    <p v-else class="pp-muted">No manifest plugins for this server role.</p>
  </PpCard>
</template>

<style scoped>
.section-title {
  margin: 0 0 0.35rem;
  font-size: 1rem;
}
.desc {
  margin: 0 0 1rem;
  font-size: 0.85rem;
}
.list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
.row {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  align-items: flex-start;
  padding: 0.65rem 0;
  border-bottom: 1px solid var(--color-border-subtle);
}
.name {
  font-weight: 600;
  display: block;
}
.meta {
  font-size: 0.75rem;
}
.blurb {
  margin: 0.35rem 0 0;
  font-size: 0.8rem;
}
</style>
