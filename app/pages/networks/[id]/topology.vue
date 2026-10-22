<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { VueFlow } from '@vue-flow/core'
import type { Connection } from '@vue-flow/core'
import { Background } from '@vue-flow/background'
import '@vue-flow/core/dist/style.css'
import '@vue-flow/core/dist/theme-default.css'
import { usePatchApi } from '~/composables/usePatchApi'

const route = useRoute()
const networkId = computed(() => route.params.id as string)
const api = usePatchApi()

const topology = ref<Awaited<ReturnType<typeof api.networkTopologyGet>> | null>(null)
const message = ref('')
const loading = ref(true)
const poolState = ref('active')
const playersPerBackend = ref(40)
const hiddenReady = ref(1)
const scalePromote = ref<number | null>(null)
const scaleLoading = ref(false)
const scarecrow = ref<{ allowed: number; dropped: number } | null>(null)

const nodes = computed(() => {
  if (!topology.value) return []
  return topology.value.nodes.map((n, i) => ({
    id: n.id,
    label: n.label,
    position: { x: n.kind === 'proxy' ? 80 : 320, y: 40 + i * 110 },
    class: `vf-node vf-node--${n.kind}`,
  }))
})

const edges = computed(() => {
  if (!topology.value) return []
  return topology.value.edges.map((e) => ({
    id: `${e.proxy_id}-${e.backend_server_id}`,
    source: e.proxy_id,
    target: e.backend_server_id,
    animated: true,
    label: e.server_alias,
  }))
})

async function onConnect(params: Connection) {
  if (!topology.value) return
  const backend = topology.value.nodes.find((n) => n.id === params.target)
  if (!backend || backend.kind === 'proxy') return
  message.value = ''
  try {
    message.value = await api.networkHotWire(networkId.value, {
      proxy_id: networkId.value,
      backend_server_id: backend.id,
      server_alias: backend.label,
      forced_hosts: [],
      priority_tier: 0,
      permission_gate: null,
      pool_state: poolState.value,
    })
    topology.value = await api.networkTopologyGet(networkId.value)
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  }
}

async function applyPoolPromotion() {
  if (scalePromote.value === null || scalePromote.value < 1) return
  scaleLoading.value = true
  message.value = ''
  try {
    const promoted = await api.gourdPoolPromote(networkId.value, {
      count: scalePromote.value,
    })
    message.value = promoted.length
      ? `Promoted Gourd routes: ${promoted.join(', ')}`
      : 'No hidden-ready routes to promote (hot-wire backends with hidden_ready first).'
    await refreshPoolScaleHint()
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  } finally {
    scaleLoading.value = false
  }
}

async function refreshScarecrowStats() {
  try {
    scarecrow.value = await api.gourdScarecrowStats(networkId.value)
  } catch {
    scarecrow.value = null
  }
}

async function refreshPoolScaleHint() {
  scaleLoading.value = true
  try {
    const [overview, topo] = await Promise.all([
      api.analyticsNetworkOverview(networkId.value),
      api.networkTopologyGet(networkId.value),
      refreshScarecrowStats(),
    ])
    topology.value = topo
    const activeBackends = topo.nodes.filter((n) => n.kind === 'backend').length
    scalePromote.value = await api.gourdPoolScaleRecommend({
      current_ccu: overview.combined_online,
      active_backends: Math.max(1, activeBackends),
      players_per_backend: playersPerBackend.value,
      hidden_ready: hiddenReady.value,
    })
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  } finally {
    scaleLoading.value = false
  }
}

onMounted(async () => {
  try {
    topology.value = await api.networkTopologyGet(networkId.value)
    await refreshPoolScaleHint()
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <div class="topology-page">
    <NetworkSubnav />
    <header class="topology-header">
      <h1 class="topology-title">Network topology</h1>
      <p class="pp-muted">
        VueFlow canvas (plangem.md §3.1). Drag from proxy to backend to hot-wire Velocity +
        Vineyard.
      </p>
    </header>
    <p v-if="loading" class="pp-muted">Loading…</p>
    <template v-else>
      <label class="pp-muted topology-pool">
        Gourd pool state on hot-wire
        <select v-model="poolState">
          <option value="active">active</option>
          <option value="hidden_ready">hidden_ready (hot spare)</option>
          <option value="warming_up">warming_up</option>
          <option value="in_game_locked">in_game_locked</option>
          <option value="draining">draining</option>
        </select>
      </label>
      <div class="topology-scale pp-muted">
        <span>Predictive scale (plangem §3.3)</span>
        <label>
          Players / backend
          <input v-model.number="playersPerBackend" type="number" min="1" max="500" />
        </label>
        <label>
          Hidden-ready spares
          <input v-model.number="hiddenReady" type="number" min="0" max="32" />
        </label>
        <button type="button" class="pp-btn pp-btn--secondary" :disabled="scaleLoading" @click="refreshPoolScaleHint">
          {{ scaleLoading ? 'Calculating…' : 'Recalculate' }}
        </button>
        <p v-if="scalePromote !== null && !scaleLoading" class="topology-scale-hint">
          Promote up to <strong>{{ scalePromote }}</strong> hidden-ready backend(s) for current CCU.
          <button
            v-if="scalePromote > 0"
            type="button"
            class="pp-btn pp-btn--primary topology-scale-apply"
            :disabled="scaleLoading"
            @click="applyPoolPromotion"
          >
            Apply promotion
          </button>
        </p>
      </div>
      <p v-if="scarecrow" class="topology-scarecrow pp-muted">
        Scarecrow Shield (plangem §6.7): {{ scarecrow.allowed }} handshakes allowed,
        {{ scarecrow.dropped }} dropped
      </p>
      <p v-if="message" class="topology-msg">{{ message }}</p>
    </template>
    <ClientOnly>
      <VueFlow
        v-if="nodes.length"
        :nodes="nodes"
        :edges="edges"
        class="topology-flow"
        fit-view-on-init
        @connect="onConnect"
      >
        <Background pattern-color="var(--color-border-subtle)" :gap="20" />
      </VueFlow>
    </ClientOnly>
  </div>
</template>

<style scoped>
.topology-page {
  padding: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  min-height: 0;
  flex: 1;
}
.topology-title {
  font-size: 1.35rem;
  font-weight: 700;
}
.topology-msg {
  color: var(--color-accent-hover);
  font-size: 0.875rem;
}
.topology-flow {
  height: 28rem;
  border-radius: var(--radius-lg);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg-raised);
}
:deep(.vf-node--proxy .vue-flow__node) {
  border-color: var(--color-accent);
}
</style>
