# Pumpkin Patch

Desktop app built with **Nuxt** (Vue) and **Tauri v2** in one project folder (`PumpkinPatch/`) — Nuxt at the app root and Rust in `src-tauri/`.

## Prerequisites

- [Node.js](https://nodejs.org/) (LTS)
- [pnpm](https://pnpm.io/)
- [Rust](https://www.rust-lang.org/tools/install)
- Platform [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

## Development

```bash
pnpm install
pnpm dev
```

Runs `tauri dev`, which starts Nuxt on port **1420** and opens the desktop window.

## Build

```bash
pnpm build
```

## Scripts

| Script | Description |
|--------|-------------|
| `pnpm dev` | Tauri dev (Nuxt + Rust) |
| `pnpm build` | Production desktop bundle |
| `pnpm frontend:dev` | Nuxt only (browser) |
| `pnpm frontend:build` | Nuxt production build (`.output/public`) |
