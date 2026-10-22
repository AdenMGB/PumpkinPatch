import { Channel, invoke } from '@tauri-apps/api/core'
import { onUnmounted, shallowRef, watch, type Ref } from 'vue'

export interface ParsedTelemetryHeader {
  tickNumber: bigint
  totalMsptMs: number
  netDecodeMs: number
  ecsMovementMs: number
  chunkGenMs: number
  redstoneTickMs: number
  wasmPluginsMs: number
  onlinePlayers: number
  loadedChunks: number
  entityCount: number
  hotChunkCount: number
  hotChunks: ChunkHeatSample[]
}

export interface ChunkHeatSample {
  dimensionId: number
  chunkX: number
  chunkZ: number
  tickCostMs: number
  entityCount: number
}

/** Matches `vineyard_protocol::pack_frame` layout. */
export function parseTelemetryFrame(raw: Uint8Array): ParsedTelemetryHeader | null {
  const view = new DataView(raw.buffer, raw.byteOffset, raw.byteLength)
  if (view.byteLength < 70) return null
  const tickNumber = view.getBigUint64(16, true)
  const hotChunkCount = view.getUint8(66)
  const hotChunks: ChunkHeatSample[] = []
  let offset = 70
  for (let i = 0; i < hotChunkCount && offset + 21 <= view.byteLength; i++) {
    const dimensionId = view.getUint8(offset)
    const chunkX = view.getInt32(offset + 1, true)
    const chunkZ = view.getInt32(offset + 5, true)
    const tickCostUs = view.getUint32(offset + 9, true)
    const entityCount = view.getUint16(offset + 13, true)
    offset += 21
    hotChunks.push({
      dimensionId,
      chunkX,
      chunkZ,
      tickCostMs: tickCostUs / 1000,
      entityCount,
    })
  }
  return {
    tickNumber,
    totalMsptMs: view.getUint32(32, true) / 1000,
    netDecodeMs: view.getUint32(36, true) / 1000,
    ecsMovementMs: view.getUint32(40, true) / 1000,
    chunkGenMs: view.getUint32(44, true) / 1000,
    redstoneTickMs: view.getUint32(48, true) / 1000,
    wasmPluginsMs: view.getUint32(52, true) / 1000,
    onlinePlayers: view.getUint16(56, true),
    loadedChunks: view.getUint32(58, true),
    entityCount: view.getUint32(62, true),
    hotChunkCount,
    hotChunks,
  }
}

export function useLiveTelemetryStream(serverId: string | Ref<string>) {
  const latestHeader = shallowRef<ParsedTelemetryHeader | null>(null)
  const msptRingBuffer = new Float32Array(1200)
  let ringIndex = 0
  let channel: Channel<number[]> | null = null

  const attach = (id: string) => {
    if (!id) return
    channel?.close()
    channel = new Channel<number[]>()
    channel.onmessage = (raw: number[]) => {
      const bytes = new Uint8Array(raw)
      const parsed = parseTelemetryFrame(bytes)
      if (!parsed) return
      msptRingBuffer[ringIndex % msptRingBuffer.length] = parsed.totalMsptMs
      ringIndex += 1
      latestHeader.value = parsed
    }
    void invoke('subscribe_live_telemetry', {
      serverId: id,
      onFrame: channel,
    })
  }

  if (typeof serverId === 'string') {
    attach(serverId)
  } else {
    watch(serverId, (id) => attach(id), { immediate: true })
  }

  onUnmounted(() => {
    channel?.close()
  })

  return { latestHeader, msptRingBuffer }
}
