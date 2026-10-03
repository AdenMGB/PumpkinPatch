# Pumpkin Patch plugins

Standalone crates that also ship with the desktop app.

## Crates

- `patch-analytics-protocol` — signed Patch Plan export format (desktop app + plugin)
- `patch-plan-analytics` — **Patch Plan** WASM plugin (Plan-inspired analytics for Pumpkin)
- `patch-network-protocol` — shared plugin message types
- `patch-hub-lobby` — lobby plugin scaffold (replace `artifacts/patch-hub-lobby.wasm` with a real WASM build from [Pumpkin plugin docs](https://docs.pumpkinmc.org/plugin-dev/introduction))

### Build Patch Plan (Pumpkin plugin)

```bash
rustup target add wasm32-wasip2
cd plugins/patch-plan-analytics
cargo build --release
cp target/wasm32-wasip2/release/patch_plan_analytics.wasm ../artifacts/patch-plan-analytics.wasm
```

Patch Plan is **auto-deployed** to every lobby/backend when a network is created (see `manifest.json` `auto_deploy`).

## Standalone build

```bash
cargo build -p patch-network-protocol
cargo build -p patch-hub-lobby
```

## App deploy

The desktop app bundles this directory as Tauri resources (`plugins/`). At runtime it loads `manifest.json` from the bundle (or from this repo path in dev), validates artifact paths, and copies listed WASM/JAR files into each instance `plugins/` folder when creating a network lobby (respecting each entry's `targets` list).
