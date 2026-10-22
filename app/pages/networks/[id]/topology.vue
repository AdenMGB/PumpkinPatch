<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { usePatchApi } from '~/composables/usePatchApi'

const route = useRoute()
const networkId = computed(() => route.params.id as string)
const api = usePatchApi()

const topology = ref<Awaited<ReturnType<typeof api.networkTopologyGet>> | null>(null)
const message = ref('')
const loading = ref(true)

onMounted(async () => {
  try {
    topology.value = await api.networkTopologyGet(networkId.value)
  } finally {
    loading.value = false
  }
})

async function wireBackend(backendId: string, alias: string) {
  message.value = ''
  try {
    const res = await api.networkHotWire(networkId.value, {
      proxy_id: networkId.value,
      backend_server_id: backendId,
      server_alias: alias,
      forced_hosts: [],
      priority_tier: 0,
      permission_gate: null,
    })
    message.value = res
    topology.value = await api.networkTopologyGet(networkId.value)
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  }
}
</script>

<template>
  <div class="topology-page">
    <header class="topology-header">
      <h1 class="topology-title">Network topology</h1>
      <p class="pp-muted">
        Vineyard routing + Velocity hot-wire (plangem.md §3.1). Drag-style wiring is simplified to
        one-click backend registration.
      </p>
    </header>

    <p v-if="loading" class="pp-muted">Loading topology…</p>
    <p v-else-if="message" class="topology-msg">{{ message }}</p>

    <div v-if="topology" class="topology-canvas" role="list">
      <div
        v-for="node in topology.nodes"
        :key="node.id"
        class="topology-node"
        :class="`topology-node--${node.kind}`"
        role="listitem"
      >
        <span class="topology-node__kind">{{ node.kind }}</span>
        <strong>{{ node.label }}</strong>
        <span v-if="node.port" class="pp-muted">:{{ node.port }}</span>
        <button
          v-if="node.kind === 'backend'"
          type="button"
          class="pp-btn topology-node__wire"
          @click="wireBackend(node.id, node.label)"
        >
          Wire to proxy
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.topology-page {
  padding: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}
.topology-title {
  font-size: 1.35rem;
  font-weight: 700;
}
.topology-msg {
  color: var(--color-accent-hover);
  font-size: 0.875rem;
}
.topology-canvas {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
}
.topology-node {
  min-width: 220px;
  padding: 1rem;
  border-radius: var(--radius-lg);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg-raised);
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  transition: all 0.2s ease;
}
.topology-node:hover {
  transform: scale(1.02);
  border-color: var(--color-accent-muted);
}
.topology-node--proxy {
  border-color: var(--color-accent);
}
.topology-node__kind {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--color-text-muted);
}
.topology-node__wire {
  margin-top: 0.5rem;
  align-self: flex-start;
}
</style>
