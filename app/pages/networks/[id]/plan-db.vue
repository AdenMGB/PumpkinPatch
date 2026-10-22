<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import type { PlanBackupEntry, PlanSyncReport } from '~/types/patch'
import { usePatchApi } from '~/composables/usePatchApi'

const route = useRoute()
const networkId = computed(() => route.params.id as string)
const api = usePatchApi()

const backups = ref<PlanBackupEntry[]>([])
const message = ref('')
const loading = ref(false)
const backupLabel = ref('sqlite')
const restorePath = ref('')
const removePlayer = ref('')
const uninstalledServer = ref('')
const importPath = ref('')
const lastSync = ref<PlanSyncReport | null>(null)

async function refreshBackups() {
  loading.value = true
  message.value = ''
  try {
    backups.value = await api.planDbListBackups(networkId.value)
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

async function runOptimize() {
  loading.value = true
  message.value = ''
  try {
    await api.planDbOptimize()
    message.value = 'SQLite VACUUM / ANALYZE completed.'
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

async function runBackup() {
  loading.value = true
  message.value = ''
  try {
    const res = await api.planDbBackup(networkId.value, backupLabel.value || undefined)
    message.value = `Backup written (${res.bytes} bytes): ${res.path}`
    await refreshBackups()
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

async function runRestore() {
  if (!restorePath.value.trim()) {
    message.value = 'Pick a backup path to restore.'
    return
  }
  loading.value = true
  message.value = ''
  try {
    await api.planDbRestore(restorePath.value.trim())
    message.value = 'Database restored from backup.'
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

async function runRemovePlayer() {
  if (!removePlayer.value.trim()) return
  loading.value = true
  message.value = ''
  try {
    const n = await api.planDbRemovePlayer(networkId.value, removePlayer.value.trim())
    message.value = `Removed player rows: ${n}`
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

async function runClearNetwork() {
  if (!confirm('Clear all Plan analytics for this network? This cannot be undone.')) return
  loading.value = true
  message.value = ''
  try {
    const n = await api.planDbClearNetwork(networkId.value)
    message.value = `Cleared ${n} rows for this network.`
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

async function runMarkUninstalled() {
  if (!uninstalledServer.value.trim()) return
  loading.value = true
  message.value = ''
  try {
    const ok = await api.planDbMarkServerUninstalled(
      networkId.value,
      uninstalledServer.value.trim(),
    )
    message.value = ok ? 'Server marked uninstalled.' : 'No matching server found.'
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

async function runImportSigned() {
  if (!importPath.value.trim()) return
  loading.value = true
  message.value = ''
  lastSync.value = null
  try {
    lastSync.value = await api.planImportSignedExport(networkId.value, importPath.value.trim())
    message.value = 'Signed Plan export imported.'
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

function pickBackup(row: PlanBackupEntry) {
  restorePath.value = row.path
}

function formatWhen(ms: number) {
  if (!ms) return '—'
  return new Date(ms).toLocaleString()
}

onMounted(() => {
  refreshBackups()
})
</script>

<template>
  <div class="page">
    <NetworkSubnav />
    <header class="page__head">
      <h1 class="page__title">Plan database</h1>
      <p class="page__lead">
        Desktop parity for indepth.md <code>/plan db *</code> — backup, restore, optimize, GDPR
        removal, and signed import for network <code>{{ networkId }}</code>.
      </p>
    </header>

    <section class="card">
      <h2 class="card__title">Maintenance</h2>
      <div class="form-grid">
        <label>
          Backup label (subfolder)
          <input v-model="backupLabel" type="text" class="input" placeholder="sqlite" />
        </label>
        <label>
          Restore from path
          <input v-model="restorePath" type="text" class="input" placeholder="/path/to/plan-backup.db" />
        </label>
      </div>
      <div class="actions">
        <button type="button" class="btn btn--primary" :disabled="loading" @click="runBackup">
          Backup SQLite
        </button>
        <button type="button" class="btn" :disabled="loading || !restorePath" @click="runRestore">
          Restore
        </button>
        <button type="button" class="btn" :disabled="loading" @click="runOptimize">
          Optimize (VACUUM)
        </button>
        <button type="button" class="btn" :disabled="loading" @click="refreshBackups">
          Refresh list
        </button>
      </div>
    </section>

    <section class="card">
      <h2 class="card__title">Backups</h2>
      <p v-if="!backups.length" class="muted">No <code>.db</code> backups under plan-backups yet.</p>
      <ul v-else class="backup-list">
        <li v-for="row in backups" :key="row.path">
          <button type="button" class="linkish" @click="pickBackup(row)">
            <code>{{ row.path }}</code>
          </button>
          <span class="muted">{{ row.bytes }} B · {{ formatWhen(row.modified_ms) }}</span>
        </li>
      </ul>
    </section>

    <section class="card">
      <h2 class="card__title">Data operations</h2>
      <div class="form-grid">
        <label>
          Remove player (name or UUID)
          <input v-model="removePlayer" type="text" class="input" />
        </label>
        <label>
          Mark server uninstalled
          <input v-model="uninstalledServer" type="text" class="input" placeholder="Lobby UUID or name" />
        </label>
        <label>
          Import signed export path
          <input v-model="importPath" type="text" class="input" placeholder="/path/to/export.json" />
        </label>
      </div>
      <div class="actions">
        <button type="button" class="btn" :disabled="loading || !removePlayer" @click="runRemovePlayer">
          Remove player
        </button>
        <button type="button" class="btn" :disabled="loading" @click="runClearNetwork">
          Clear network data
        </button>
        <button
          type="button"
          class="btn"
          :disabled="loading || !uninstalledServer"
          @click="runMarkUninstalled"
        >
          Mark server uninstalled
        </button>
        <button type="button" class="btn" :disabled="loading || !importPath" @click="runImportSigned">
          Import signed export
        </button>
      </div>
      <pre v-if="lastSync" class="mono">{{ JSON.stringify(lastSync, null, 2) }}</pre>
    </section>

    <p v-if="message" class="banner">{{ message }}</p>
  </div>
</template>

<style scoped>
.page__head {
  margin-bottom: 1rem;
}
.page__title {
  font-size: 1.5rem;
  margin: 0 0 0.35rem;
}
.page__lead {
  color: var(--color-text-muted);
  margin: 0;
  max-width: 52rem;
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
.form-grid {
  display: grid;
  gap: 0.75rem;
  margin-bottom: 0.75rem;
}
.form-grid label {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  font-size: 0.85rem;
  font-weight: 600;
}
.input {
  font-weight: 400;
  padding: 0.45rem 0.6rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-surface-elevated);
  color: var(--color-text);
}
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}
.btn {
  padding: 0.45rem 0.85rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-surface-hover);
  cursor: pointer;
  font-weight: 600;
  font-size: 0.85rem;
}
.btn--primary {
  background: var(--color-accent);
  color: var(--color-on-accent, #fff);
  border-color: transparent;
}
.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.banner {
  margin-top: 1rem;
  padding: 0.65rem 0.85rem;
  border-radius: var(--radius-md);
  background: var(--color-accent-muted);
}
.backup-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}
.backup-list li {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  align-items: flex-start;
}
.linkish {
  background: none;
  border: none;
  padding: 0;
  cursor: pointer;
  text-align: left;
  color: var(--color-accent-hover);
}
.muted {
  color: var(--color-text-muted);
  font-size: 0.8rem;
}
.mono {
  font-size: 0.75rem;
  overflow-x: auto;
  margin-top: 0.75rem;
}
</style>
