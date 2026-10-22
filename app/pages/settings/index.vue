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
