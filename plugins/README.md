# Pumpkin Patch plugins

Standalone crates that also ship with the desktop app.

## Crates

- `patch-network-protocol` — shared plugin message types
- `patch-hub-lobby` — lobby plugin scaffold (replace `artifacts/patch-hub-lobby.wasm` with a real WASM build from [Pumpkin plugin docs](https://docs.pumpkinmc.org/plugin-dev/introduction))

## Standalone build

```bash
cargo build -p patch-network-protocol
cargo build -p patch-hub-lobby
```

## App deploy

The desktop app bundles this directory as Tauri resources (`plugins/`). At runtime it loads `manifest.json` from the bundle (or from this repo path in dev), validates artifact paths, and copies listed WASM/JAR files into each instance `plugins/` folder when creating a network lobby (respecting each entry's `targets` list).
