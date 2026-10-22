import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { DownloadProgress, LogLine } from '~/types/patch'

export const networkLogs = ref<LogLine[]>([])
export const downloadProgress = ref<DownloadProgress | null>(null)

export function pushNetworkLog(line: LogLine) {
  networkLogs.value = [...networkLogs.value.slice(-4999), line]
}

export function mergePersistedLogs(lines: LogLine[]) {
  if (!lines.length) return
  const merged = [...lines, ...networkLogs.value]
  networkLogs.value = merged.slice(-5000)
}

export default defineNuxtPlugin(async () => {
  if (!isTauri()) {
    console.info('[patch] browser-only dev — Tauri commands are unavailable')
    return
  }

  const api = usePatchApi()
  try {
    await api.initialize()
  } catch (e) {
    console.warn('[patch] initialize_state failed', e)
  }

  await listen<LogLine>('network-log', (event) => {
    pushNetworkLog(event.payload)
  })

  await listen<DownloadProgress>('network-download-progress', (event) => {
    downloadProgress.value = event.payload
    if (event.payload.total != null && event.payload.downloaded >= event.payload.total) {
      window.setTimeout(() => {
        downloadProgress.value = null
      }, 1200)
    }
  })
})
