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
    })
    topology.value = await api.networkTopologyGet(networkId.value)
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  }
}

onMounted(async () => {
  try {
    topology.value = await api.networkTopologyGet(networkId.value)
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
    <p v-else-if="message" class="topology-msg">{{ message }}</p>
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
