<script setup lang="ts">
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import {
  ClipboardDocumentIcon,
  Cog6ToothIcon,
  PlayIcon,
  PlusIcon,
  StopIcon,
  TrashIcon,
} from '@heroicons/vue/24/outline'

const route = useRoute()
const api = usePatchApi()
const queryClient = useQueryClient()
const id = computed(() => route.params.id as string)

const { data: network, isLoading } = useQuery({
  queryKey: ['network', id],
  queryFn: () => api.networkGet(id.value),
})

const { data: ping } = useQuery({
  queryKey: ['network-ping', id],
  queryFn: () => api.networkPingHub(id.value),
  refetchInterval: 8000,
  enabled: computed(() => network.value?.status === 'running'),
})

const { data: health, isLoading: healthLoading } = useQuery({
  queryKey: ['network-health', id],
  queryFn: () => api.networkHealthGet(id.value),
  refetchInterval: 15000,
})

const deleteOpen = ref(false)
const autoRestart = computed({
  get: () => network.value?.network.auto_restart ?? false,
  set: (v: boolean) => {
    api.networkSetAutoRestart(id.value, v).then(() => {
      queryClient.invalidateQueries({ queryKey: ['network', id] })
    })
  },
})

const backupMutation = useMutation({
  mutationFn: () => api.networkExportBackupDefault(id.value),
})

const startMutation = useMutation({
  mutationFn: () => api.networkStart(id.value),
  onSuccess: () => queryClient.invalidateQueries({ queryKey: ['network', id] }),
})

const stopMutation = useMutation({
  mutationFn: () => api.networkStop(id.value),
  onSuccess: () => queryClient.invalidateQueries({ queryKey: ['network', id] }),
})

const deleteMutation = useMutation({
  mutationFn: () => api.networkDelete(id.value),
  onSuccess: () => navigateTo('/library'),
})

const newBackendName = ref('')
const addBackendMutation = useMutation({
  mutationFn: () =>
    api.serverAddBackend(id.value, {
      name: newBackendName.value,
      minecraft_version: network.value?.servers[0]?.minecraft_version,
    }),
  onSuccess: () => {
    newBackendName.value = ''
    queryClient.invalidateQueries({ queryKey: ['network', id] })
  },
})

const removeServerMutation = useMutation({
  mutationFn: (serverId: string) => api.serverRemove(id.value, serverId),
  onSuccess: () => queryClient.invalidateQueries({ queryKey: ['network', id] }),
})

async function copyJoin() {
  if (network.value) await navigator.clipboard.writeText(network.value.join_address)
}

function confirmDeleteNetwork() {
  deleteOpen.value = true
}

const backendServers = computed(() =>
  network.value?.servers.filter((s) => s.role === 'backend') ?? [],
)

async function copyCommand(cmd: string) {
  await navigator.clipboard.writeText(cmd)
}
</script>

<template>
  <div class="pp-page">
    <NetworkSubnav />

    <p v-if="isLoading" class="pp-muted">Loading…</p>
    <template v-else-if="network">
      <PpPageHeader :title="network.network.name" description="Status, servers, and quick actions">
        <template #actions>
          <PpBadge :tone="network.status === 'running' ? 'success' : 'neutral'">
            {{ network.status }}
          </PpBadge>
          <PpButton variant="secondary" @click="copyJoin">
            <ClipboardDocumentIcon class="ico" aria-hidden="true" />
            {{ network.join_address }}
          </PpButton>
          <NuxtLink :to="`/networks/${network.network.id}/settings`">
            <PpButton variant="secondary">
              <Cog6ToothIcon class="ico" aria-hidden="true" />
              Settings
            </PpButton>
          </NuxtLink>
          <PpButton
            v-if="network.status !== 'running'"
            :loading="startMutation.isPending.value"
            @click="startMutation.mutate()"
          >
            <PlayIcon class="ico" aria-hidden="true" />
            Start
          </PpButton>
          <PpButton
            v-else
            variant="danger"
            :loading="stopMutation.isPending.value"
            @click="stopMutation.mutate()"
          >
            <StopIcon class="ico" aria-hidden="true" />
            Stop
          </PpButton>
          <PpIconButton
            label="Delete network"
            variant="danger"
            :disabled="network.status === 'running'"
            @click="confirmDeleteNetwork"
          >
            <TrashIcon aria-hidden="true" />
          </PpIconButton>
        </template>
      </PpPageHeader>

      <NetworkHealthPanel :report="health" :loading="healthLoading" />

      <PpCard v-if="network.network.last_crash_source" padding="md" class="crash">
        <PpBadge tone="danger">Last crash</PpBadge>
        <span class="pp-muted">{{ network.network.last_crash_source }}</span>
      </PpCard>

      <div v-if="ping?.online" class="ping-banner">
        <PpBadge tone="success">Hub online</PpBadge>
        <span class="pp-muted">
          {{ ping.players_online ?? 0 }} / {{ ping.players_max ?? '?' }} players
        </span>
        <span v-if="ping.version" class="pp-muted">· {{ ping.version }}</span>
      </div>

      <PpCard v-if="backendServers.length" padding="lg" class="switch-card">
        <h2 class="section-title">Switch game servers</h2>
        <p class="pp-muted section-hint">
          Join via the hub, then in chat run Velocity&apos;s
          <code class="pp-code">/server &lt;name&gt;</code> (names match the table below).
        </p>
        <ul class="switch-list">
          <li v-for="s in backendServers" :key="s.id" class="switch-row">
            <span class="server-name">{{ s.name }}</span>
            <code class="pp-code">/server {{ s.velocity_name }}</code>
            <PpButton size="sm" variant="ghost" @click="copyCommand(`/server ${s.velocity_name}`)">
              Copy
            </PpButton>
          </li>
        </ul>
      </PpCard>

      <PpCard padding="lg">
        <h2 class="section-title">Servers</h2>
        <p class="pp-muted section-hint">
          Configure gameplay, versions, and hub options on the
          <NuxtLink :to="`/networks/${network.network.id}/settings`" class="link">Settings</NuxtLink>
          page.
        </p>
        <ul class="server-list">
          <li v-for="s in network.servers" :key="s.id" class="server-row">
            <div class="server-info">
              <span class="server-name">{{ s.name }}</span>
              <PpBadge tone="neutral">{{ s.role }}</PpBadge>
              <span class="pp-muted meta">MC {{ s.minecraft_version }}</span>
              <span class="pp-muted port">:{{ s.game_port }}</span>
            </div>
            <div class="server-actions">
              <NuxtLink :to="`/networks/${network.network.id}/settings?server=${s.id}`">
                <PpButton size="sm" variant="ghost">Configure</PpButton>
              </NuxtLink>
              <PpIconButton
                v-if="s.role !== 'lobby'"
                label="Remove server"
                variant="danger"
                :disabled="network.status === 'running'"
                @click="removeServerMutation.mutate(s.id)"
              >
                <TrashIcon aria-hidden="true" />
              </PpIconButton>
            </div>
          </li>
        </ul>

        <div class="ops-row">
          <label class="auto-restart">
            <input v-model="autoRestart" type="checkbox" />
            Auto-restart on crash (max 5 attempts)
          </label>
          <PpButton variant="secondary" :loading="backupMutation.isPending.value" @click="backupMutation.mutate()">
            Backup network
          </PpButton>
          <p v-if="backupMutation.data" class="pp-muted backup-path">{{ backupMutation.data }}</p>
        </div>

        <div class="add-backend">
          <PpInput v-model="newBackendName" label="New backend" placeholder="minigames" />
          <PpButton
            variant="secondary"
            :disabled="!newBackendName.trim()"
            :loading="addBackendMutation.isPending.value"
            @click="addBackendMutation.mutate()"
          >
            <PlusIcon class="ico" aria-hidden="true" />
            Add
          </PpButton>
        </div>
      </PpCard>
      <PpModal
        :open="deleteOpen"
        title="Delete network?"
        confirm-label="Delete"
        danger
        :loading="deleteMutation.isPending.value"
        @close="deleteOpen = false"
        @confirm="deleteMutation.mutate(); deleteOpen = false"
      >
        <p>This removes all server files and cannot be undone.</p>
      </PpModal>
    </template>
  </div>
</template>

<style scoped>
.ico {
  width: 1rem;
  height: 1rem;
}
.ping-banner {
  display: flex;
  gap: 0.75rem;
  align-items: center;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}
.section-title {
  margin: 0 0 0.35rem;
  font-size: 1rem;
}
.section-hint {
  margin: 0 0 1rem;
  font-size: 0.85rem;
}
.link {
  color: var(--color-accent-hover);
  text-decoration: none;
}
.link:hover {
  text-decoration: underline;
}
.server-list {
  list-style: none;
  margin: 0 0 1rem;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.server-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  flex-wrap: wrap;
  padding: 0.65rem 0.75rem;
  border-radius: var(--radius-md);
  background: var(--color-bg);
  border: 1px solid var(--color-border-subtle);
}
.server-info {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}
.server-name {
  font-weight: 600;
  font-size: 0.9rem;
}
.meta {
  font-size: 0.8rem;
}
.port {
  font-size: 0.8rem;
}
.server-actions {
  display: flex;
  gap: 0.35rem;
  align-items: center;
}
.add-backend {
  display: flex;
  gap: 0.5rem;
  align-items: flex-end;
  border-top: 1px solid var(--color-border-subtle);
  padding-top: 1rem;
}
.add-backend .pp-field {
  flex: 1;
}
.switch-card {
  margin-bottom: 1rem;
}
.switch-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.switch-row {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  flex-wrap: wrap;
}
.crash {
  margin-bottom: 1rem;
  display: flex;
  flex-wrap: wrap;
  gap: 0.65rem;
  align-items: center;
}
.auto-restart {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.85rem;
}
.ops-row {
  margin-bottom: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}
.backup-path {
  font-size: 0.75rem;
  word-break: break-all;
}
</style>
