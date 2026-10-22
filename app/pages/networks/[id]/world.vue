<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import * as THREE from 'three'
import { usePatchApi } from '~/composables/usePatchApi'
import { useLiveTelemetryStream } from '~/composables/useBinaryChannel'

const route = useRoute()
const api = usePatchApi()
const networkId = route.params.id as string
const mountEl = ref<HTMLDivElement | null>(null)
const lobbyServerId = ref('')
const { latestHeader: latest } = useLiveTelemetryStream(lobbyServerId)

let renderer: THREE.WebGLRenderer | null = null
let frameId = 0
const chunkMeshes = new Map<string, THREE.Mesh>()

onMounted(async () => {
  const summary = await api.networkGet(networkId)
  const lobby = summary.servers.find((s) => s.role === 'lobby')
  lobbyServerId.value = lobby?.id ?? summary.servers[0]?.id ?? ''

  if (!mountEl.value) return
  const scene = new THREE.Scene()
  scene.background = new THREE.Color(0x111827)
  const camera = new THREE.PerspectiveCamera(55, 1, 0.1, 500)
  camera.position.set(12, 14, 12)
  camera.lookAt(0, 0, 0)

  const light = new THREE.DirectionalLight(0xffffff, 1.1)
  light.position.set(6, 12, 4)
  scene.add(light)
  scene.add(new THREE.AmbientLight(0x404040))

  const chunkGeom = new THREE.BoxGeometry(1, 0.15, 1)
  for (let x = -4; x <= 4; x++) {
    for (let z = -4; z <= 4; z++) {
      const mat = new THREE.MeshStandardMaterial({
        color: new THREE.Color().setHSL(0.08, 0.5, 0.4),
      })
      const mesh = new THREE.Mesh(chunkGeom, mat)
      mesh.position.set(x * 1.1, 0, z * 1.1)
      scene.add(mesh)
      chunkMeshes.set(`${x}:${z}`, mesh)
    }
  }

  renderer = new THREE.WebGLRenderer({ antialias: true })
  renderer.setPixelRatio(window.devicePixelRatio)
  mountEl.value.appendChild(renderer.domElement)

  const resize = () => {
    if (!mountEl.value || !renderer) return
    const { clientWidth, clientHeight } = mountEl.value
    renderer.setSize(clientWidth, clientHeight)
    camera.aspect = clientWidth / clientHeight
    camera.updateProjectionMatrix()
  }
  resize()
  window.addEventListener('resize', resize)

  const animate = () => {
    frameId = requestAnimationFrame(animate)
    camera.position.y = 14 + Math.sin(Date.now() / 4000) * 0.5
    renderer?.render(scene, camera)
  }
  animate()
})

watch(latest, (frame: typeof latest.value) => {
  if (!frame) return
  for (const sample of frame.hotChunks) {
    const key = `${sample.chunkX}:${sample.chunkZ}`
    const mesh = chunkMeshes.get(key)
    if (!mesh) continue
    const heat = Math.min(1, sample.tickCostMs / 12)
    const mat = mesh.material as THREE.MeshStandardMaterial
    mat.color.setHSL(0.05 + heat * 0.55, 0.85, 0.35 + heat * 0.25)
    mesh.scale.y = 1 + heat * 4
    mesh.position.y = (mesh.scale.y * 0.15) / 2
  }
})

onUnmounted(() => {
  cancelAnimationFrame(frameId)
  renderer?.dispose()
  renderer?.domElement.remove()
})
</script>

<template>
  <div class="world-page">
    <NetworkSubnav />
    <PpPageHeader
      title="3D world inspector"
      :description="
        latest
          ? `Live MSPT ${latest!.totalMsptMs.toFixed(2)} ms · ${latest!.hotChunkCount} hot chunks (Vineyard telemetry)`
          : 'Waiting for telemetry from lobby server…'
      "
    />
    <div ref="mountEl" class="world-canvas" />
  </div>
</template>

<style scoped>
.world-page {
  padding: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  flex: 1;
  min-height: 0;
}
.world-canvas {
  flex: 1;
  min-height: 22rem;
  border-radius: var(--radius-lg);
  border: 1px solid var(--color-border-subtle);
  overflow: hidden;
}
</style>
