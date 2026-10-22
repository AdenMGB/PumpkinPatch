<script setup lang="ts">
import { useQuery, useMutation, useQueryClient } from '@tanstack/vue-query'
import type { AppSettings } from '~/types/patch'

const api = usePatchApi()
const queryClient = useQueryClient()

const { data: settings } = useQuery({
  queryKey: ['settings'],
  queryFn: () => api.settingsGet(),
})

const form = ref<AppSettings>({
  java_path: 'java',
  bind_host: '127.0.0.1',
  pumpkin_channel_default: 'nightly',
  plan_http_enabled: true,
  plan_http_port: 8804,
  harvest_duckdb_enabled: true,
  gourd_enabled: false,
})

watch(
  settings,
  (s) => {
    if (s) form.value = { ...s }
  },
  { immediate: true },
)

const saveMutation = useMutation({
  mutationFn: () => api.settingsSet(form.value),
  onSuccess: () => queryClient.invalidateQueries({ queryKey: ['settings'] }),
})

const detectMutation = useMutation({
  mutationFn: () => api.settingsDetectJava(),
  onSuccess: (res) => {
    if (res.resolved_path) form.value.java_path = res.resolved_path
  },
})
</script>

<template>
  <div class="pp-page settings">
    <PpPageHeader
      title="Settings"
      description="Global defaults for new networks and the Velocity JVM."
    />
    <PpCard padding="lg" class="form-card">
      <form class="form" @submit.prevent="saveMutation.mutate()">
        <PpInput
          v-model="form.java_path"
          label="Java executable"
          hint="Use java for auto-detect (JDK 17+). Full path is saved after detection."
        />
        <PpButton
          type="button"
          variant="secondary"
          :loading="detectMutation.isPending.value"
          @click="detectMutation.mutate()"
        >
          Detect Java
        </PpButton>
        <p v-if="detectMutation.data?.candidates?.length" class="pp-muted candidates">
          Found:
          {{ detectMutation.data.candidates.slice(0, 3).join(' · ') }}
        </p>
        <PpInput
          v-model="form.bind_host"
          label="Bind host"
          hint="Shown in join addresses (127.0.0.1 or your LAN IP)."
        />
        <PpInput
          v-model="form.pumpkin_channel_default"
          label="Default Pumpkin channel"
          hint="GitHub release tag, usually nightly."
        />
        <label class="settings-check">
          <input v-model="form.plan_http_enabled" type="checkbox" />
          Enable Plan-compatible HTTP API (`/v1` on bind host)
        </label>
        <PpInput
          v-model.number="form.plan_http_port"
          label="Plan HTTP port"
          hint="Default 8804 (indepth.md embedded webserver)."
        />
        <label class="settings-check">
          <input v-model="form.harvest_duckdb_enabled" type="checkbox" />
          HarvestMetrics JSON mirror sync
        </label>
        <label class="settings-check">
          <input v-model="form.gourd_enabled" type="checkbox" />
          Prefer native Gourd proxy (experimental)
        </label>
        <PpButton type="submit" :loading="saveMutation.isPending.value">Save settings</PpButton>
      </form>
    </PpCard>
  </div>
</template>

<style scoped>
.settings {
  max-width: 32rem;
}
.form-card {
  margin-top: 0.5rem;
}
.settings-check {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.875rem;
  color: var(--color-text);
}
.form {
  display: flex;
  flex-direction: column;
  gap: 1.15rem;
}
.candidates {
  margin: -0.5rem 0 0;
  font-size: 0.75rem;
  word-break: break-all;
}
</style>
