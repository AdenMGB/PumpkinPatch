import { Channel, invoke } from '@tauri-apps/api/core'
import { onUnmounted, shallowRef } from 'vue'

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
}

export function useLiveTelemetryStream(serverId: string) {
  const latestHeader = shallowRef<ParsedTelemetryHeader | null>(null)
  const msptRingBuffer = new Float32Array(1200)
  let ringIndex = 0

  const channel = new Channel<number[]>()
  channel.onmessage = (raw: number[]) => {
    const rawBuffer = new Uint8Array(raw).buffer
    const view = new DataView(rawBuffer)
    if (view.byteLength < 56) return
    const tickNumber = view.getBigUint64(16, true)
    const totalMsptUs = view.getUint32(32, true)
    const totalMsptMs = totalMsptUs / 1000.0
    msptRingBuffer[ringIndex % msptRingBuffer.length] = totalMsptMs
    ringIndex += 1
    latestHeader.value = {
      tickNumber,
      totalMsptMs,
      netDecodeMs: view.getUint32(36, true) / 1000.0,
      ecsMovementMs: view.getUint32(40, true) / 1000.0,
      chunkGenMs: view.getUint32(44, true) / 1000.0,
      redstoneTickMs: view.getUint32(48, true) / 1000.0,
      wasmPluginsMs: view.getUint32(52, true) / 1000.0,
      onlinePlayers: view.getUint16(56, true),
      loadedChunks: view.getUint32(60, true),
      entityCount: view.getUint32(64, true),
    }
  }

  void invoke('subscribe_live_telemetry', {
    serverId,
    onFrame: channel,
  })

  onUnmounted(() => {
    channel.close()
  })

  return { latestHeader, msptRingBuffer }
}
