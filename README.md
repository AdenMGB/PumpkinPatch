# Pumpkin Patch

Desktop hub for **PumpkinMC server networks**: Velocity proxy + Pumpkin lobby/backends, managed from a Modrinth-inspired Nuxt + Tauri app.

## Architecture

- `crates/patch-core` — SQLite, Plan/HarvestMetrics ingest, `/v1` query engine, Vineyard bus host sampler
- `crates/patch-plan-http` — Plan-compatible Axum REST (`8804`, indepth.md §5.3)
- `crates/vineyard-*`, `gourd-proxy`, `stem-agent`, `harvest-metrics` — plangem.md phase-1 platform crates
- `src-tauri` — Tauri commands (networks, Plan analytics, topology hot-wire, binary telemetry IPC)
- `app/` — Nuxt 4 UI (`/library`, `/networks/:id/analytics`, `/topology`, …)
- `plugins/` — `patch-hub-lobby`, `patch-plan-analytics`, shared protocol crates (WASM via manifest)

Specs: `indepth.md` (Plan port), `plangem.md` (PumpkinPatch platform roadmap).

## Development

```bash
pnpm install
pnpm dev:all      # Build bundled WASM plugins, then Tauri + Nuxt on :1420
pnpm dev          # App only (skip plugin rebuild)
pnpm plugins:build
```

```bash
cargo test -p patch-core
```

## Join flow

1. Create a network (hub port + backends).
2. Start the network (downloads Pumpkin + Velocity on first run).
3. Players join **`bind_host:hub_port`** (default `127.0.0.1:25565`).

Configure Java path for Velocity under **Settings**.
