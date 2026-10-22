<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRoute } from 'vue-router'
import type { PackManifest } from '~/types/patch'
import { usePatchApi } from '~/composables/usePatchApi'

const route = useRoute()
const networkId = computed(() => route.params.id as string)
const api = usePatchApi()

const baseDir = ref('')
const overlayDir = ref('')
const outputDir = ref('')
const message = ref('')
const manifest = ref<PackManifest | null>(null)
const fingerprint = ref('')
const loading = ref(false)

async function runMergePreview() {
  loading.value = true
  message.value = ''
  manifest.value = null
  try {
    manifest.value = await api.patchworkMergePackDirs(baseDir.value, overlayDir.value)
    message.value = `Merged manifest: ${manifest.value.description} (${Object.keys(manifest.value.files).length} files indexed)`
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

async function runMaterialize() {
  if (!outputDir.value.trim()) {
    message.value = 'Output directory is required to materialize.'
    return
  }
  loading.value = true
  message.value = ''
  try {
    manifest.value = await api.patchworkMaterializeMergedPack(
      baseDir.value,
      overlayDir.value,
      outputDir.value,
    )
    message.value = `Materialized pack to ${outputDir.value}`
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

async function runFingerprint() {
  loading.value = true
  fingerprint.value = ''
  try {
    fingerprint.value = await api.patchworkFingerprintDir(baseDir.value || overlayDir.value)
  } catch (e) {
    message.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="page">
    <NetworkSubnav />
    <header class="page__head">
      <h1 class="page__title">PatchWork</h1>
      <p class="page__lead">
        Resource pack merge &amp; materialize for network <code>{{ networkId }}</code> (plangem §5.2).
        Overlay wins on conflicts; SHA-256 asset index for CAS layers.
      </p>
    </header>

    <section class="card">
      <h2 class="card__title">Pack directories</h2>
      <div class="form-grid">
        <label>
          Base pack path
          <input v-model="baseDir" type="text" class="input" placeholder="/path/to/base-pack" />
        </label>
        <label>
          Overlay pack path
          <input v-model="overlayDir" type="text" class="input" placeholder="/path/to/overlay-pack" />
        </label>
        <label>
          Output path (materialize)
          <input v-model="outputDir" type="text" class="input" placeholder="/path/to/merged-out" />
        </label>
      </div>
      <div class="actions">
        <button type="button" class="btn" :disabled="loading || !baseDir || !overlayDir" @click="runMergePreview">
          Preview merge index
        </button>
        <button type="button" class="btn btn--primary" :disabled="loading || !baseDir || !overlayDir || !outputDir" @click="runMaterialize">
          Materialize merged pack
        </button>
        <button type="button" class="btn" :disabled="loading || (!baseDir && !overlayDir)" @click="runFingerprint">
          Fingerprint directory
        </button>
      </div>
      <p v-if="message" class="banner">{{ message }}</p>
      <p v-if="fingerprint" class="mono">Fingerprint: {{ fingerprint }}</p>
    </section>

    <section v-if="manifest" class="card">
      <h2 class="card__title">Manifest</h2>
      <p>Format {{ manifest.pack_format }} — {{ manifest.description }}</p>
      <p class="muted">{{ Object.keys(manifest.files).length }} indexed files</p>
      <ul class="file-list">
        <li v-for="(hash, path) in manifest.files" :key="path">
          <code>{{ path }}</code> <span class="muted">{{ hash.slice(0, 18) }}…</span>
        </li>
      </ul>
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
}
.form-grid label {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--color-text-muted);
}
.input {
  padding: 0.45rem 0.6rem;
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-subtle);
  background: var(--color-bg);
}
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin-top: 0.75rem;
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
.mono {
  font-family: ui-monospace, monospace;
  font-size: 0.8rem;
  word-break: break-all;
}
.muted {
  color: var(--color-text-muted);
  font-size: 0.85rem;
}
.file-list {
  max-height: 16rem;
  overflow: auto;
  font-size: 0.8rem;
  padding-left: 1.1rem;
}
</style>
