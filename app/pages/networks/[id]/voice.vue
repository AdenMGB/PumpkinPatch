<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import type { TendrilVoicePeer, TendrilVoiceStats, VoiceAudioOutputDevice } from '~/types/patch'
import { usePatchApi } from '~/composables/usePatchApi'

const route = useRoute()
const networkId = computed(() => route.params.id as string)
const api = usePatchApi()

const bind = ref('0.0.0.0:24454')
const proximity = ref(48)
const message = ref('')
const loading = ref(false)
const stats = ref<TendrilVoiceStats | null>(null)
const peers = ref<TendrilVoicePeer[]>([])
const flushUuid = ref('')
const flushResult = ref<string | null>(null)
const devices = ref<VoiceAudioOutputDevice[]>([])
const selectedDevice = ref('')
const earX = ref(0)
const earY = ref(64)
const earZ = ref(0)
const earDimension = ref('minecraft:overworld')
const monitorOn = ref(false)
const evidenceDir = ref('')

let pollTimer: ReturnType<typeof setInterval> | null = null

async function refresh() {
  try {
    stats.value = await api.tendrilVoiceStats()
    peers.value = await api.tendrilVoiceListPeers()
  } catch {
    /* SFU may not be started */
  }
}

async function startSfu() {
  loading.value = true
  message.value = ''
  try {
    message.value = await api.tendrilVoiceStart(bind.value, proximity.value)
    await refresh()
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

async function loadDevices() {
  try {
    devices.value = await api.voiceDeckListDevices()
    if (!selectedDevice.value && devices.value.length) {
      selectedDevice.value = devices.value[0].id
    }
  } catch {
    devices.value = []
  }
}

async function toggleMonitor() {
  message.value = ''
  try {
    if (monitorOn.value) {
      await api.voiceDeckStopMonitor()
      monitorOn.value = false
      message.value = 'Admin audio monitor stopped.'
    } else {
      const msg = await api.voiceDeckStartMonitor(
        earX.value,
        earY.value,
        earZ.value,
        earDimension.value,
        selectedDevice.value || undefined,
      )
      monitorOn.value = true
      message.value = msg
    }
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  }
}

async function playEvidence() {
  if (!evidenceDir.value.trim()) return
  message.value = ''
  try {
    const n = await api.voiceDeckPlayEvidence(
      evidenceDir.value.trim(),
      selectedDevice.value || undefined,
    )
    message.value = `Queued ${n} Opus frames from evidence directory.`
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  }
}

async function flushEvidence() {
  if (!flushUuid.value.trim()) return
  flushResult.value = null
  message.value = ''
  try {
    const res = await api.tendrilVoiceFlushReport(flushUuid.value.trim())
    flushResult.value = `Wrote ${res.captured_packets} packets (${res.total_bytes} bytes) to ${res.output_dir}`
    await refresh()
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  }
}

onMounted(() => {
  refresh()
  loadDevices()
  pollTimer = setInterval(refresh, 3000)
})

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer)
})
</script>

<template>
  <div class="page">
    <NetworkSubnav />
    <header class="page__head">
      <h1 class="page__title">TendrilVoice</h1>
      <p class="page__lead">
        Spatial UDP SFU for network <code>{{ networkId }}</code> — rolling 60s Opus evidence per speaker
        (plangem §5.1). Gourd backends publish `VP` Vineyard position frames; the SFU subscribes on start for roaming-safe proximity.
      </p>
    </header>

    <section class="card">
      <h2 class="card__title">Start SFU</h2>
      <div class="form-row">
        <label>
          Bind address
          <input v-model="bind" type="text" class="input" />
        </label>
        <label>
          Proximity (blocks)
          <input v-model.number="proximity" type="number" min="8" max="256" class="input input--narrow" />
        </label>
        <button type="button" class="btn btn--primary" :disabled="loading" @click="startSfu">
          {{ loading ? 'Starting…' : 'Start / restart' }}
        </button>
      </div>
      <p v-if="message" class="banner">{{ message }}</p>
    </section>

    <section v-if="stats" class="card stats">
      <h2 class="card__title">Live stats</h2>
      <dl class="stats__grid">
        <div><dt>Peers</dt><dd>{{ stats.peers }}</dd></div>
        <div><dt>Packets in</dt><dd>{{ stats.packets_in }}</dd></div>
        <div><dt>Packets out</dt><dd>{{ stats.packets_out }}</dd></div>
        <div><dt>Bytes in</dt><dd>{{ stats.bytes_in }}</dd></div>
        <div><dt>Forwards</dt><dd>{{ stats.forwards }}</dd></div>
        <div><dt>Spatial drops</dt><dd>{{ stats.spatial_drops }}</dd></div>
      </dl>
    </section>

    <section class="card">
      <h2 class="card__title">Active sessions</h2>
      <p v-if="!peers.length" class="muted">No UDP peers yet. Clients handshake with TV framing or SVC opaque payloads.</p>
      <table v-else class="table">
        <thead>
          <tr>
            <th>Player UUID</th>
            <th>UDP address</th>
            <th>Dimension</th>
            <th>Buffered packets</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="p in peers" :key="p.addr">
            <td><code class="uuid">{{ p.player_uuid }}</code></td>
            <td>{{ p.addr }}</td>
            <td>{{ p.dimension }}</td>
            <td>{{ p.packets_buffered }}</td>
          </tr>
        </tbody>
      </table>
    </section>

    <section class="card">
      <h2 class="card__title">Admin live audio deck</h2>
      <p class="muted">
        Drop an ear at a world coordinate (cpal output). Speakers within SFU proximity play through your
        desktop headphones — no Minecraft client required (plangem §5.1).
      </p>
      <div class="form-row">
        <label>
          Output device
          <select v-model="selectedDevice" class="input">
            <option value="">Default</option>
            <option v-for="d in devices" :key="d.id" :value="d.id">{{ d.name }}</option>
          </select>
        </label>
        <label>
          X
          <input v-model.number="earX" type="number" class="input input--narrow" />
        </label>
        <label>
          Y
          <input v-model.number="earY" type="number" class="input input--narrow" />
        </label>
        <label>
          Z
          <input v-model.number="earZ" type="number" class="input input--narrow" />
        </label>
        <label class="grow">
          Dimension
          <input v-model="earDimension" type="text" class="input" />
        </label>
        <button type="button" class="btn" :class="{ 'btn--primary': !monitorOn }" @click="toggleMonitor">
          {{ monitorOn ? 'Stop monitor' : 'Start monitor' }}
        </button>
      </div>
      <div class="form-row">
        <label class="grow">
          Evidence directory (replay)
          <input v-model="evidenceDir" type="text" class="input" placeholder="/tmp/pumpkinpatch-voice-evidence/..." />
        </label>
        <button type="button" class="btn" :disabled="!evidenceDir.trim()" @click="playEvidence">
          Play evidence
        </button>
      </div>
    </section>

    <section class="card">
      <h2 class="card__title">Flush moderation evidence</h2>
      <p class="muted">
        Dump the speaker&apos;s rolling 60s RAM buffer to disk (frame_*.opus + manifest.json) for ticket review.
      </p>
      <div class="form-row">
        <label class="grow">
          Speaker UUID
          <input v-model="flushUuid" type="text" class="input" placeholder="00000000-0000-0000-0000-000000000000" />
        </label>
        <button type="button" class="btn" :disabled="!flushUuid.trim()" @click="flushEvidence">
          Flush to disk
        </button>
      </div>
      <p v-if="flushResult" class="banner banner--ok">{{ flushResult }}</p>
    </section>
  </div>
</template>

<style scoped>
.page__head {
  margin-bottom: 1.25rem;
}
.page__title {
  font-size: 1.5rem;
  margin: 0 0 0.35rem;
}
.page__lead {
  margin: 0;
  color: var(--color-text-muted);
  font-size: 0.92rem;
  line-height: 1.5;
}
.card {
  background: var(--color-surface);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-lg);
  padding: 1rem 1.15rem;
  margin-bottom: 1rem;
}
.card__title {
  font-size: 1rem;
  margin: 0 0 0.75rem;
}
.form-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  align-items: flex-end;
}
.form-row label {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--color-text-muted);
}
.grow {
  flex: 1;
  min-width: 14rem;
}
.input {
  padding: 0.45rem 0.6rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg);
  color: var(--color-text);
  font-size: 0.9rem;
}
.input--narrow {
  width: 6rem;
}
.btn {
  padding: 0.5rem 1rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-surface-hover);
  cursor: pointer;
  font-weight: 600;
}
.btn--primary {
  background: var(--color-accent);
  color: var(--color-on-accent, #fff);
  border-color: transparent;
}
.banner {
  margin-top: 0.75rem;
  padding: 0.5rem 0.75rem;
  border-radius: var(--radius-md);
  background: var(--color-accent-muted);
  font-size: 0.85rem;
}
.banner--ok {
  background: color-mix(in srgb, var(--color-success, #22c55e) 18%, transparent);
}
.stats__grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(8rem, 1fr));
  gap: 0.75rem;
  margin: 0;
}
.stats__grid dt {
  font-size: 0.75rem;
  color: var(--color-text-muted);
}
.stats__grid dd {
  margin: 0.15rem 0 0;
  font-size: 1.25rem;
  font-weight: 700;
}
.muted {
  color: var(--color-text-muted);
  font-size: 0.88rem;
}
.table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.85rem;
}
.table th,
.table td {
  text-align: left;
  padding: 0.4rem 0.5rem;
  border-bottom: 1px solid var(--color-border-subtle);
}
.uuid {
  font-size: 0.75rem;
}
</style>
