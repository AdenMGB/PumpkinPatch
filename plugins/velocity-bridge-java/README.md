# pumpkinpatch-velocity-bridge

Java Velocity companion plugin (plangem.md Phase 1). Connects to the Pumpkin Patch desktop control plane on `bind_host:25589` and applies dynamic `register_route` commands.

Build with a standard Velocity 3.x plugin toolchain and drop the JAR into your network hub `plugins/` folder. The Rust `velocity-bridge` crate in this repo implements the line-delimited JSON protocol.
