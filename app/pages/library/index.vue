<script setup lang="ts">
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import {
  ArrowPathIcon,
  ClipboardDocumentIcon,
  PlayIcon,
  StopIcon,
  TrashIcon,
} from '@heroicons/vue/24/outline'
import type { NetworkSummary, NetworkRuntimeStatus } from '~/types/patch'

const api = usePatchApi()
const queryClient = useQueryClient()

const { data: networks, isLoading, refetch } = useQuery({
  queryKey: ['networks'],
  queryFn: () => api.networkList(),
})

const deleteTarget = ref<NetworkSummary | null>(null)

const startMutation = useMutation({
  mutationFn: (id: string) => api.networkStart(id),
  onSuccess: () => queryClient.invalidateQueries({ queryKey: ['networks'] }),
})

const stopMutation = useMutation({
  mutationFn: (id: string) => api.networkStop(id),
  onSuccess: () => queryClient.invalidateQueries({ queryKey: ['networks'] }),
})

const deleteMutation = useMutation({
  mutationFn: (id: string) => api.networkDelete(id),
  onSuccess: () => queryClient.invalidateQueries({ queryKey: ['networks'] }),
})

function statusTone(s: NetworkRuntimeStatus) {
  if (s === 'running') return 'success'
  if (s === 'error') return 'danger'
  if (s === 'starting' || s === 'stopping') return 'warning'
  return 'neutral'
}

async function copyJoin(address: string) {
  await navigator.clipboard.writeText(address)
}

function confirmDelete(item: NetworkSummary) {
  deleteTarget.value = item
}
</script>

<template>
  <div class="pp-page">
    <PpPageHeader
      title="Networks"
      description="One join address per hub — Velocity proxy + Pumpkin backends."
    >
      <template #actions>
        <PpButton variant="secondary" @click="refetch()">
          <ArrowPathIcon class="ico" aria-hidden="true" />
          Refresh
        </PpButton>
        <NuxtLink to="/networks/new">
          <PpButton>Create network</PpButton>
        </NuxtLink>
      </template>
    </PpPageHeader>

    <p v-if="isLoading" class="pp-muted">Loading networks…</p>

    <PpCard v-else-if="!networks?.length" padding="lg" class="empty">
      <h2>No networks yet</h2>
      <p class="pp-muted">Create a Velocity hub with a lobby and game backends in one wizard.</p>
      <NuxtLink to="/networks/new">
        <PpButton size="lg">Create your first network</PpButton>
      </NuxtLink>
    </PpCard>

    <div v-else class="grid">
      <PpCard v-for="item in networks" :key="item.network.id" padding="lg" hover class="card">
        <div class="card-head">
          <div>
            <h2>{{ item.network.name }}</h2>
            <p class="pp-muted meta">
              {{ item.servers.length }} servers · {{ item.network.hub_type }} ·
              {{ item.servers[0]?.minecraft_version ?? '1.21.4' }}
            </p>
          </div>
          <PpBadge :tone="statusTone(item.status)">{{ item.status }}</PpBadge>
        </div>

        <div class="join-row">
          <span class="join-label">Join</span>
          <code class="pp-code">{{ item.join_address }}</code>
          <PpIconButton label="Copy join address" @click="copyJoin(item.join_address)">
            <ClipboardDocumentIcon aria-hidden="true" />
          </PpIconButton>
        </div>

        <div class="card-actions">
          <NuxtLink :to="`/networks/${item.network.id}`">
            <PpButton variant="secondary">Overview</PpButton>
          </NuxtLink>
          <NuxtLink :to="`/networks/${item.network.id}/settings`">
            <PpButton variant="ghost">Settings</PpButton>
          </NuxtLink>
          <PpButton
            v-if="item.status !== 'running'"
            :loading="startMutation.isPending.value"
            @click="startMutation.mutate(item.network.id)"
          >
            <PlayIcon class="ico" aria-hidden="true" />
            Start
          </PpButton>
          <PpButton
            v-else
            variant="danger"
            :loading="stopMutation.isPending.value"
            @click="stopMutation.mutate(item.network.id)"
          >
            <StopIcon class="ico" aria-hidden="true" />
            Stop
          </PpButton>
          <PpIconButton
            label="Delete network"
            variant="danger"
            :disabled="item.status === 'running'"
            @click="confirmDelete(item)"
          >
            <TrashIcon aria-hidden="true" />
          </PpIconButton>
        </div>
      </PpCard>
    </div>

    <PpModal
      v-if="deleteTarget"
      :open="!!deleteTarget"
      title="Delete network?"
      confirm-label="Delete"
      danger
      :loading="deleteMutation.isPending.value"
      @close="deleteTarget = null"
      @confirm="deleteMutation.mutate(deleteTarget!.network.id); deleteTarget = null"
    >
      <p>
        Delete &quot;{{ deleteTarget?.network.name }}&quot;? This removes all server data and cannot
        be undone.
      </p>
    </PpModal>
  </div>
</template>

<style scoped>
.ico {
  width: 1rem;
  height: 1rem;
}
.empty h2 {
  margin: 0 0 0.5rem;
}
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(20rem, 1fr));
  gap: 1rem;
}
.card-head {
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
  align-items: flex-start;
}
h2 {
  margin: 0;
  font-size: 1.15rem;
}
.meta {
  margin: 0.25rem 0 0;
  font-size: 0.8rem;
}
.join-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin: 1rem 0;
  flex-wrap: wrap;
}
.join-label {
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  color: var(--color-text-muted);
}
.card-actions {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
  align-items: center;
}
</style>
