import { isTauri } from '@tauri-apps/api/core'
import { type as osType } from '@tauri-apps/plugin-os'

export type DesktopOs = 'windows' | 'macos' | 'linux' | 'unknown'

function detectOs(): DesktopOs {
  if (!import.meta.client || !isTauri()) return 'unknown'
  try {
    const kind = osType()
    if (kind === 'windows' || kind === 'macos' || kind === 'linux') return kind
  } catch {
    /* ignore */
  }
  return 'unknown'
}

export function useTauriPlatform() {
  const inTauri = computed(() => import.meta.client && isTauri())
  const os = ref<DesktopOs>('unknown')

  const usesCustomWindowChrome = computed(
    () => inTauri.value && (os.value === 'windows' || os.value === 'linux'),
  )

  onMounted(() => {
    os.value = detectOs()
  })

  if (import.meta.client && isTauri()) {
    os.value = detectOs()
  }

  return { inTauri, os, usesCustomWindowChrome }
}
