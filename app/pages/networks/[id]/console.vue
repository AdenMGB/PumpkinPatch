<script setup lang="ts">
import { useMutation, useQuery } from '@tanstack/vue-query'
import { networkLogs, pushNetworkLog } from '~/plugins/patch-init.client'

const route = useRoute()
const api = usePatchApi()
const id = computed(() => route.params.id as string)
const filter = ref('')
const command = ref('')
const target = ref('hub')
const consoleRef = ref<HTMLElement | null>(null)
const historyKey = computed(() => `patch-console-history-${id.value}`)
const history = ref<string[]>([])
const favorites = ref<string[]>([])

onMounted(() => {
  if (import.meta.client) {
    try {
      history.value = JSON.parse(localStorage.getItem(historyKey.value) ?? '[]')
      favorites.value = JSON.parse(localStorage.getItem(`${historyKey.value}-fav`) ?? '[]')
    } catch {
      history.value = []
    }
  }
})

function pushHistory(cmd: string) {
  const trimmed = cmd.trim()
  if (!trimmed) return
  history.value = [trimmed, ...history.value.filter((c) => c !== trimmed)].slice(0, 30)
  if (import.meta.client) {
    localStorage.setItem(historyKey.value, JSON.stringify(history.value))
  }
}

const { data: network } = useQuery({
  queryKey: ['network', id],
  queryFn: () => api.networkGet(id.value),
})

const targets = computed(() => {
  const list = [{ value: 'hub', label: 'Velocity hub' }]
  for (const s of network.value?.servers ?? []) {
    list.push({
      value: s.id,
      label: `${s.name} (${s.role})`,
    })
  }
  return list
})

const lines = computed(() => {
  const all = networkLogs.value.filter((l) => l.network_id === id.value)
  if (!filter.value.trim()) return all
  const q = filter.value.toLowerCase()
  return all.filter((l) => l.source.toLowerCase().includes(q) || l.line.toLowerCase().includes(q))
})

const text = computed(() =>
  lines.value.map((l) => `[${l.source}] ${l.line}`).join('\n') ||
  'No logs yet. Start the network to stream hub, lobby, and backend output.',
)

watch(
  () => lines.value.length,
  () => {
    nextTick(() => {
      const el = consoleRef.value
      if (el) el.scrollTop = el.scrollHeight
    })
  },
)

const sendMutation = useMutation({
  mutationFn: () =>
    api.networkConsoleSend(id.value, { target: target.value, command: command.value }),
  onSuccess: (res) => {
    const label =
      targets.value.find((t) => t.value === target.value)?.label ?? target.value
    pushNetworkLog({
      network_id: id.value,
      source: `console:${label}`,
      line: `> ${command.value.trim()}`,
    })
    if (res.output) {
      pushNetworkLog({
        network_id: id.value,
        source: `console:${label}`,
        line: res.output,
      })
    }
    pushHistory(command.value)
    command.value = ''
  },
  onError: (err: Error) => {
    pushNetworkLog({
      network_id: id.value,
      source: 'console:error',
      line: err.message ?? String(err),
    })
  },
})

function onSubmit() {
  if (!command.value.trim() || network.value?.status !== 'running') return
  sendMutation.mutate()
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    onSubmit()
  }
}
</script>

<template>
  <div class="pp-page console-page">
    <NetworkSubnav />
    <PpPageHeader
      title="Console"
      description="Live logs plus commands — Velocity via proxy console, Pumpkin servers via RCON."
    />

    <div class="target-tabs" data-tauri-drag-region-exclude>
      <button
        v-for="t in targets"
        :key="t.value"
        type="button"
        class="tab"
        :class="{ active: target === t.value }"
        @click="target = t.value"
      >
        {{ t.label }}
      </button>
    </div>

    <div class="toolbar" data-tauri-drag-region-exclude>
      <PpInput v-model="filter" label="Filter logs" placeholder="velocity, lobby, survival…" class="filter" />
    </div>

    <pre ref="consoleRef" class="console">{{ text }}</pre>

    <form class="command-bar" data-tauri-drag-region-exclude @submit.prevent="onSubmit">
      <input
        v-model="command"
        type="text"
        class="command-input"
        placeholder="help, list, server survival, say hello…"
        :disabled="network?.status !== 'running' || sendMutation.isPending.value"
        autocomplete="off"
        @keydown="onKeydown"
      />
      <PpButton
        type="submit"
        :loading="sendMutation.isPending.value"
        :disabled="network?.status !== 'running' || !command.trim()"
      >
        Send
      </PpButton>
    </form>
    <p v-if="network && network.status !== 'running'" class="pp-muted hint">
      Start the network to send commands.
    </p>
    <p v-else class="pp-muted hint">
      Pumpkin servers use RCON on port <code class="pp-code">game_port + 2000</code> (e.g. lobby
      25566 → 27566). Velocity commands show up in the log stream.
    </p>

    <div v-if="history.length" class="history">
      <span class="pp-muted">Recent:</span>
      <button
        v-for="h in history.slice(0, 8)"
        :key="h"
        type="button"
        class="hist-btn"
        @click="command = h"
      >
        {{ h }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.console-page {
  display: flex;
  flex-direction: column;
  min-height: min(70vh, 40rem);
}
.toolbar {
  display: flex;
  gap: 1rem;
  flex-wrap: wrap;
  margin-bottom: 0.75rem;
}
.target-select {
  min-width: 14rem;
}
.filter {
  flex: 1;
  min-width: 12rem;
}
.console {
  flex: 1;
  min-height: 12rem;
  max-height: min(55vh, 32rem);
  overflow: auto;
  background: #080810;
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-lg);
  padding: 1rem 1.15rem;
  font-family: ui-monospace, 'Cascadia Code', monospace;
  font-size: 0.78rem;
  line-height: 1.5;
  margin: 0 0 0.75rem;
  white-space: pre-wrap;
  color: #d4d4e8;
}
.command-bar {
  display: flex;
  gap: 0.5rem;
  align-items: stretch;
}
.command-input {
  flex: 1;
  padding: 0.6rem 0.85rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg-elevated);
  color: var(--color-text);
  font-family: ui-monospace, 'Cascadia Code', monospace;
  font-size: 0.85rem;
}
.command-input:focus {
  outline: none;
  border-color: var(--color-accent);
  box-shadow: 0 0 0 3px var(--color-accent-muted);
}
.hint {
  margin: 0.65rem 0 0;
  font-size: 0.8rem;
}
.target-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  margin-bottom: 0.75rem;
}
.tab {
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg);
  color: var(--color-text);
  padding: 0.35rem 0.65rem;
  border-radius: var(--radius-md);
  font-size: 0.8rem;
  cursor: pointer;
  transition: all 0.2s ease-in-out;
}
.tab.active {
  border-color: var(--color-accent);
  background: var(--color-accent-muted);
}
.tab:hover {
  transform: scale(1.02);
}
.history {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  align-items: center;
  margin-top: 0.65rem;
}
.hist-btn {
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg-elevated);
  color: var(--color-text-muted);
  font-size: 0.72rem;
  padding: 0.2rem 0.45rem;
  border-radius: var(--radius-md);
  cursor: pointer;
}
</style>
