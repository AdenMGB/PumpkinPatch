# Pumpkin Patch

Desktop hub for **PumpkinMC server networks**: Velocity proxy + Pumpkin lobby/backends, managed from a Modrinth-inspired Nuxt + Tauri app.

## Architecture

- `crates/patch-core` — SQLite, downloads, `velocity.toml` / proxy config generation, process supervisor, ping
- `src-tauri` — Tauri commands (`network_*`, `settings_*`, …)
- `app/` — Nuxt 4 UI (`/library`, `/networks/new`, …)
- `plugins/` — Bundled WASM (`patch-hub-lobby`, `patch-plan-analytics`), profiles, manifest

## Platform matrix

| Feature | Windows | Linux | macOS |
|--------|---------|-------|-------|
| Pumpkin download | Yes | Yes | Yes |
| Tauri desktop build | Yes | CI | CI |
| Auto-updater | Configure signing in release workflow | Same | Same |

See `docs/operations.md` and `docs/hub-plugin.md`.

## Development

```bash
pnpm install
pnpm dev          # Tauri + Nuxt on :1420
```

```bash
cargo test -p patch-core
```

## Join flow

1. Create a network (hub port + backends).
2. Start the network (downloads Pumpkin + Velocity on first run).
3. Players join **`bind_host:hub_port`** (default `127.0.0.1:25565`).

Configure Java path for Velocity under **Settings**.
