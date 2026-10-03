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
  java_min_major: 21,
})

const javaTest = useMutation({
  mutationFn: () => api.javaValidate(),
})
const cacheClear = useMutation({
  mutationFn: () => api.cacheClear(),
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
        <PpInput
          v-model="form.bind_host"
          label="Bind host"
          hint="127.0.0.1 = local only. Use 0.0.0.0 to listen on all interfaces (LAN). Tailscale/VPN: use your tailnet IP."
        />
        <PpInput
          v-model.number="form.java_min_major"
          label="Minimum Java major version"
          type="number"
        />
        <PpButton type="button" variant="secondary" :loading="javaTest.isPending.value" @click="javaTest.mutate()">
          Validate Java
        </PpButton>
        <p v-if="javaTest.data" class="pp-muted">{{ javaTest.data }}</p>
        <p v-if="javaTest.error" class="error">{{ (javaTest.error as Error).message }}</p>
        <PpInput
          v-model="form.pumpkin_channel_default"
          label="Default Pumpkin channel"
          hint="GitHub release tag, usually nightly."
        />
        <PpButton type="submit" :loading="saveMutation.isPending.value">Save settings</PpButton>
        <PpButton type="button" variant="ghost" :loading="cacheClear.isPending.value" @click="cacheClear.mutate()">
          Clear download cache
        </PpButton>
        <p v-if="cacheClear.data != null" class="pp-muted">Cleared {{ cacheClear.data }} bytes</p>
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
.form {
  display: flex;
  flex-direction: column;
  gap: 1.15rem;
}
.error {
  color: var(--color-danger);
  font-size: 0.85rem;
}
</style>
