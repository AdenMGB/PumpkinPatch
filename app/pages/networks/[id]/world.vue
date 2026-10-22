<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import * as THREE from 'three'

const route = useRoute()
const networkId = computed(() => route.params.id as string)
const mountEl = ref<HTMLDivElement | null>(null)

let renderer: THREE.WebGLRenderer | null = null
let frameId = 0

onMounted(async () => {
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
        color: new THREE.Color().setHSL(0.08 + (x + z) * 0.02, 0.7, 0.45),
      })
      const mesh = new THREE.Mesh(chunkGeom, mat)
      mesh.position.set(x * 1.1, 0, z * 1.1)
      scene.add(mesh)
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
      description="Three.js live chunk scaffold (plangem.md Phase 2). Wire Pumpkin hook SDK telemetry for per-chunk MSPT heat."
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
