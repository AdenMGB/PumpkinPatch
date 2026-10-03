<script setup lang="ts">
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import type { HubSettings, UpdateHubSettingsRequest } from '~/types/patch'

const props = defineProps<{ networkId: string }>()
const api = usePatchApi()
const queryClient = useQueryClient()

const { data: hub, isLoading } = useQuery({
  queryKey: ['hub-settings', props.networkId],
  queryFn: () => api.hubGetSettings(props.networkId),
})

const form = ref<HubSettings>({
  bind: '127.0.0.1:25565',
  motd: '',
  online_mode: true,
  show_max_players: 500,
  player_info_forwarding: 'modern',
})

watch(
  hub,
  (h) => {
    if (h) form.value = { ...h }
  },
  { immediate: true },
)

const saveMutation = useMutation({
  mutationFn: () => {
    const patch: UpdateHubSettingsRequest = {
      bind: form.value.bind,
      motd: form.value.motd,
      online_mode: form.value.online_mode,
      show_max_players: Number(form.value.show_max_players),
      player_info_forwarding: form.value.player_info_forwarding,
    }
    return api.hubUpdateSettings(props.networkId, patch)
  },
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['hub-settings', props.networkId] })
  },
})
</script>

<template>
  <PpCard padding="lg" class="hub-panel">
    <h2 class="section-title">Velocity hub</h2>
    <p class="pp-muted section-desc">
      Public join proxy — edits <code class="pp-code">velocity.toml</code>. Restart the network to
      apply bind changes.
    </p>
    <p v-if="isLoading" class="pp-muted">Loading hub settings…</p>
    <form v-else class="form-grid" @submit.prevent="saveMutation.mutate()">
      <PpInput v-model="form.bind" label="Bind address" hint="Players connect to this host:port" />
      <PpInput v-model="form.motd" label="MOTD" />
      <PpInput
        v-model.number="form.show_max_players"
        label="Show max players"
        type="number"
      />
      <PpSelect
        v-model="form.player_info_forwarding"
        label="Player info forwarding"
        :options="[
          { value: 'modern', label: 'Modern (Velocity)' },
          { value: 'legacy', label: 'Legacy' },
          { value: 'none', label: 'None' },
        ]"
      />
      <label class="toggle">
        <input v-model="form.online_mode" type="checkbox" data-tauri-drag-region-exclude />
        <span>Online mode (Microsoft auth)</span>
      </label>
      <div class="form-actions">
        <PpButton type="submit" :loading="saveMutation.isPending.value">Save hub settings</PpButton>
      </div>
    </form>
  </PpCard>
</template>

<style scoped>
.section-title {
  margin: 0 0 0.35rem;
  font-size: 1.1rem;
}
.section-desc {
  margin: 0 0 1.25rem;
  font-size: 0.85rem;
}
.form-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr));
  gap: 1rem;
}
.toggle {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.875rem;
  color: var(--color-text-muted);
}
.form-actions {
  grid-column: 1 / -1;
}
</style>
