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

The app reads `manifest.json` and copies listed artifacts into each instance `plugins/` folder when creating a network lobby.
