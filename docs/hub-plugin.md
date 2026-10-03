# Hub lobby plugin

`patch-hub-lobby` is a bundled WASM plugin deployed to the lobby server.

## Server list

Pumpkin Patch writes `plugins/data/patch-hub-lobby/server-list.json` whenever network instances sync (start, add/remove backend). The payload matches `patch-network-protocol` `ServerListPayload`.

## Player commands

- `/servers` or `/hub` — list backends
- `/join <velocity_name>` — reminds players to run `/server <name>` on Velocity

Lobby join messages also list servers automatically.

## Velocity

Ensure `velocity.toml` `[try]` includes `lobby` and each backend is registered under `[servers]`. Patch regenerates this file on topology changes.
