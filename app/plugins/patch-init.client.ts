import { listen } from '@tauri-apps/api/event'
import type { LogLine } from '~/types/patch'

export const networkLogs = ref<LogLine[]>([])

export function pushNetworkLog(line: LogLine) {
  networkLogs.value = [...networkLogs.value.slice(-4999), line]
}

export default defineNuxtPlugin(async () => {
  const api = usePatchApi()
  try {
    await api.initialize()
  } catch (e) {
    console.warn('[patch] initialize_state failed', e)
  }

  await listen<LogLine>('network-log', (event) => {
    pushNetworkLog(event.payload)
  })
})
