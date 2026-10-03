<script setup lang="ts">
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import type { ServerRecord } from '~/types/patch'

const props = withDefaults(
  defineProps<{ server: ServerRecord; fullPage?: boolean }>(),
  { fullPage: false },
)
const api = usePatchApi()
const queryClient = useQueryClient()

const { data: settings, isLoading } = useQuery({
  queryKey: ['server-settings', props.server.id],
  queryFn: () => api.serverGetSettings(props.server.id),
})

const form = ref({
  bind_address: '',
  online_mode: false,
  max_players: 100,
  view_distance: 12,
  simulation_distance: 10,
  motd: '',
  commands_enabled: true,
})

const minecraftVersion = ref(props.server.minecraft_version)
const pumpkinChannel = ref(props.server.pumpkin_channel)

watch(
  () => props.server,
  (s) => {
    minecraftVersion.value = s.minecraft_version
    pumpkinChannel.value = s.pumpkin_channel
  },
  { immediate: true },
)

watch(
  settings,
  (s) => {
    if (s) form.value = { ...s }
  },
  { immediate: true },
)

const saveMutation = useMutation({
  mutationFn: () =>
    api.serverUpdateSettings(props.server.id, {
      ...form.value,
      max_players: Number(form.value.max_players),
      view_distance: Number(form.value.view_distance),
      simulation_distance: Number(form.value.simulation_distance),
    }),
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['server-settings', props.server.id] })
  },
})

const versionMutation = useMutation({
  mutationFn: (v: string) => api.serverSetMinecraftVersion(props.server.id, v),
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ['network', props.server.network_id] })
  },
})

function onMinecraftVersionChange(v: string) {
  minecraftVersion.value = v
  if (v !== props.server.minecraft_version) {
    versionMutation.mutate(v)
  }
}
</script>

<template>
  <PpCard padding="lg" class="settings-panel">
    <h2 class="section-title">{{ server.name }}</h2>
    <p class="pp-muted section-desc">
      {{ server.role }} · Velocity name <code class="pp-code">{{ server.velocity_name }}</code> ·
      game port {{ server.game_port }}
    </p>

    <section class="block">
      <h3 class="block-title">Compatibility</h3>
      <PpVersionSelect
        :model-value="minecraftVersion"
        label="Minecraft version"
        hint="Used for Modrinth plugin search and network metadata."
        @update:model-value="onMinecraftVersionChange"
      />
      <PpInput
        v-model="pumpkinChannel"
        class="channel-field"
        label="Pumpkin binary channel"
        hint="GitHub release tag (e.g. nightly). Changing requires re-download on next start."
        disabled
      />
    </section>

    <section class="block">
      <h3 class="block-title">Gameplay & networking</h3>
      <p v-if="isLoading" class="pp-muted">Loading pumpkin.patch.toml…</p>
      <form v-else class="form-grid" @submit.prevent="saveMutation.mutate()">
        <PpInput
          v-model="form.bind_address"
          label="Bind address"
          hint="Locked to this server's game port (127.0.0.1:port). Velocity uses the hub port separately."
          disabled
        />
        <PpInput v-model="form.motd" label="MOTD" />
        <PpInput v-model.number="form.max_players" label="Max players" type="number" />
        <PpInput v-model.number="form.view_distance" label="View distance" type="number" />
        <PpInput
          v-model.number="form.simulation_distance"
          label="Simulation distance"
          type="number"
        />
        <label class="toggle">
          <input v-model="form.online_mode" type="checkbox" data-tauri-drag-region-exclude />
          <span>Online mode</span>
        </label>
        <label class="toggle">
          <input v-model="form.commands_enabled" type="checkbox" data-tauri-drag-region-exclude />
          <span>Commands enabled</span>
        </label>
        <div class="form-actions">
          <PpButton type="submit" :loading="saveMutation.isPending.value">
            Save Pumpkin settings
          </PpButton>
        </div>
      </form>
    </section>

    <p v-if="fullPage" class="pp-muted path-hint">
      Instance path:
      <code class="pp-code">{{ server.data_path }}</code>
    </p>
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
.block {
  margin-bottom: 1.5rem;
  padding-bottom: 1.25rem;
  border-bottom: 1px solid var(--color-border-subtle);
}
.block:last-of-type {
  border-bottom: none;
  margin-bottom: 0;
  padding-bottom: 0;
}
.block-title {
  margin: 0 0 0.85rem;
  font-size: 0.8rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--color-text-muted);
}
.channel-field {
  margin-top: 1rem;
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
.path-hint {
  margin-top: 1.25rem;
  font-size: 0.75rem;
  word-break: break-all;
}
</style>
