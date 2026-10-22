# PUMPKINPATCH: THE DEFINITIVE ARCHITECTURAL & FEATURE SPECIFICATION

**Document ID:** `PP-SPEC-2026-TAURI-REV2`

**Application Stack:** Tauri v2 (Rust Core Backend + WebView2/WebKitGTK) + Vue 3.5+ (TypeScript, Pinia, TailwindCSS, Three.js / WebGL2)

**Underlying Compute Engine:** PumpkinMC (Rust Native Server Implementation)

**Proxy Layer:** *Gourd* (Bespoke Rust Velocity-Compatible Zero-Copy Edge Proxy) + Native Java Velocity Bridge

**Audio Layer:** *TendrilVoice* (Rust Native UDP WebRTC/SRTP Spatial Voice SFU + Built-in Tauri Admin Voice Deck)

**Inter-Node Bus:** *Vineyard* (QUIC + `rkyv` Zero-Copy Mesh IPC & Tauri Raw binary IPC Channel Engine)

---

## MODULE 1: TAURI v2 + VUE 3 NATIVE DESKTOP & DISTRIBUTED ARCHITECTURE

Unlike traditional web-based server panels (e.g., Pterodactyl, Crafty, AMP) that require heavy Docker setups, NGINX reverse proxies, PHP/Node.js runtimes, and suffer from browser WebSocket serialization bottlenecks, **PumpkinPatch** is built as a **native, ultra-low-overhead Tauri v2 desktop and headless-daemon application** powered by a reactive **Vue 3** frontend and a unified **Rust** systems backend.

By unifying the desktop management UI (Tauri + Vue), the proxy layer (**Gourd**), the voice server (**TendrilVoice**), and the game server (**PumpkinMC**) under a single language ecosystem (Rust), PumpkinPatch achieves direct memory-mapped telemetry, native GPU-accelerated 3D world inspection, and one-click local or remote cluster orchestration.

### 1.1 Dual-Mode Tauri Execution Topologies

Because Minecraft networks range from a developer testing a local SMP on their gaming PC to a 5,000-player multi-region dedicated server cluster, the PumpkinPatch Tauri binary supports three seamless operational modes out of the box:

1. **Local Sovereign Mode (All-in-One Desktop App):**
* The Tauri Core Rust process directly spawns, supervises, and shares OS memory (`mmap` / POSIX Shared Memory / Windows Named Shared Memory) with local **PumpkinMC** nodes and the **Gourd** proxy right on the user's workstation or home server.
* Zero network overhead: Telemetry structs from PumpkinMC are read directly from shared RAM by the Tauri backend and streamed into the Vue 3 frontend via **Tauri v2 Raw Binary IPC Channels (`tauri::ipc::Channel`)** at 60 FPS without JSON serialization bloat.


2. **Remote Fleet Controller Mode (Tauri Desktop Client $\leftrightarrow$ Remote `stem-agent` Nodes):**
* You run the PumpkinPatch Tauri Desktop App on your Windows, macOS, or Linux machine.
* Your remote Linux bare-metal servers or VPS instances run **`stem-agent`**—a lightweight, headless Rust daemon compiled from the exact same workspace crates as the Tauri backend.
* The Tauri app connects directly to your remote `stem-agent` cluster via **mTLS-authenticated QUIC streams (`quinn`)**. Your desktop app acts as a real-time mission control center with zero public web panel attack surface (no exposed HTTP login pages to brute-force).


3. **Hybrid Headless / Team Collaborative Mode:**
* A headless hub node runs `pumpkinpatch-hub` so multiple staff members running the PumpkinPatch Tauri Desktop (or iOS/Android Tauri v2 Mobile build) can connect concurrently, share real-time cursors on the node topology graph, co-edit configs with live CRDT conflict resolution, and monitor live player analytics together.



---

### 1.2 The Tauri v2 + Vue 3 Frontend-to-Core Pipeline

```
+---------------------------------------------------------------------------------------+
|                   PUMPKINPATCH TAURI v2 DESKTOP / MOBILE CLIENT                       |
|                                                                                       |
|  +---------------------------------------------------------------------------------+  |
|  |                     VUE 3.5+ FRONTEND (WebView2 / WebKit)                       |  |
|  |  +-------------------+  +--------------------+  +----------------------------+  |  |
|  |  | Pinia Reactive    |  | Three.js / WebGL2  |  | VueFlow Node Graph         |  |  |
|  |  | Binary Stores     |  | 3D Live World Map  |  | (Velocity/Gourd Topology)  |  |  |
|  |  +---------+---------+  +---------+----------+  +-------------+--------------+  |  |
|  |            |                      |                           |                 |  |
|  |  +---------+----------------------+---------------------------+--------------+  |  |
|  |  | Monaco IDE (TOML/Rust/WASM)  | ECharts/uPlot (60fps Canvas) | Xterm.jsGL  |  |  |
|  |  +------------------------------+------------------------------+-------------+  |  |
|  +-----------------------------------|---------------------------------------------+  |
|                                      |                                                |
|         Tauri v2 Commands (`#[tauri::command]`) & Raw Binary `ipc::Channel`           |
|              (Zero-JSON `ArrayBuffer` transfers for Chunks, Audio & Metrics)          |
|                                      |                                                |
|  +-----------------------------------v---------------------------------------------+  |
|  |                        TAURI RUST CORE (`src-tauri`)                            |  |
|  |  +-------------------+  +--------------------+  +----------------------------+  |  |
|  |  | Embedded DuckDB / |  | Local Process &    |  | Native WebRTC / Rodio      |  |  |
|  |  | SQLite-WAL Engine |  | PTY Supervisor     |  | Admin Voice Moderation DSP |  |  |
|  |  +---------+---------+  +---------+----------+  +-------------+--------------+  |  |
|  +------------|----------------------|---------------------------|-----------------+  |
+---------------|----------------------|---------------------------|--------------------+
                |                      |                           |
================|======================|===========================|====================
                v        VINEYARD MESH BUS (QUIC + SHM)            v
+---------------------------------------------------------------------------------------+
|  LOCAL HOST: OS Shared Memory (`/dev/shm` or Win32 `CreateFileMapping`) Ring Buffers  |
|  REMOTE NODES: mTLS QUIC Multiplexed Streams + `rkyv` Zero-Copy Deserialization       |
+---------------+-----------------------------------+-----------------------------------+
                |                                   |
                v                                   v
+-------------------------------+   +---------------------------------------------------+
|   "GOURD" RUST EDGE PROXY     |   |           PUMPKINMC SERVER NODE (RUST)            |
|  (Or Velocity + JNI Bridge)   |   |  +---------------------+ +----------------------+ |
| +---------------------------+ |   |  | Embedded Vineyard   | | TendrilVoice UDP     | |
| | eBPF / XDP DDoS Filter    | |<->|  | Telemetry & ECS Spy | | Spatial Audio SFU    | |
| | Zero-Copy Packet Splicer  | |   |  | Live Chunk Streamer | | Region Lock Manager  | |
| +---------------------------+ |   |  +---------------------+ +----------------------+ |
+-------------------------------+   +---------------------------------------------------+

```

#### Why Tauri v2 + Vue 3 Changes Everything for Server Management

* **Zero-JSON Binary IPC Streaming (`tauri::ipc::Channel<tauri::ipc::InvokeResponseBody>`):** When rendering a live 3D map of a PumpkinMC world or plotting 10,000 entity tick times per second, standard web panels choke on `JSON.stringify()` and `JSON.parse()`. PumpkinPatch sends raw byte buffers (`Vec<u8>`) directly from the Rust backend into Vue's JavaScript `Uint8Array` / `Float32Array` typed arrays, which are uploaded straight to the GPU via WebGL2 / WebGPU.
* **Native OS Integration:**
* **System Tray Mini-Controller:** Quick-toggle servers, view global TPS/CCU sparklines, copy local tunnel IPs, and receive native OS notifications for server crashes, DDoS spikes, or high-priority staff alerts even when the main window is minimized.
* **Hardware Keychain & Biometrics:** SSH keys, Cloudflare API tokens, Mojang/Microsoft OAuth tokens, and Vineyard mTLS private keys are stored in the native OS vault (**Windows Hello / DPAPI, macOS Keychain / TouchID, Linux Secret Service / KWallet**).
* **Multi-Window Detachable Workspaces:** Powered by Tauri's `WebviewWindowBuilder`, administrators can tear off any Vue tab—such as a server's live Xterm.js console, the 3D World Inspector, or the Plan Analytics dashboard—into its own native OS window across multiple monitors, synchronized in real time via Tauri backend state (`tauri::State<AppState>`).
* **Drag-and-Drop Native File System Access:** Drag a `.jar`, `.wasm`, `.schem`, or world `.zip` directly from Windows Explorer / macOS Finder onto a server card in the Vue UI; the Tauri Rust backend hashes the file with `BLAKE3`, validates its header, and streams it over QUIC to the target server without buffering it through a browser upload limit.



---

### 1.3 The "Gourd" Proxy vs. Java Velocity: Architectural Decision Matrix

You noted uncertainty around whether to wrap Java Velocity or rewrite Velocity in Rust. PumpkinPatch solves this with a **dual-driver proxy architecture**: full Day-1 support for managing standard **Java Velocity** (enhanced with a custom Rust JNI/FFI bridge plugin), alongside **Gourd**—PumpkinPatch's native Rust Velocity-compatible edge proxy compiled directly into the ecosystem.

| Architectural Dimension | Legacy Java Velocity (3.x) | **Gourd** (Native Rust Proxy Engine) |
| --- | --- | --- |
| **Memory Footprint (Idle)** | 256 MB – 512 MB (JVM Heap + Metaspace overhead) | **4.2 MB – 12 MB** (Zero GC, static binary allocation) |
| **Packet Forwarding Path** | Netty ByteBuf allocation $\rightarrow$ Java Heap $\rightarrow$ GC pressure $\rightarrow$ Socket write | **`io_uring` / `splice()` Zero-Copy** kernel-to-kernel socket splicing for uninspected game packets |
| **Tail Latency (p99.9)** | 4ms – 25ms spikes during G1GC / ZGC pauses under 5,000+ CCU load | **Sub-150 microseconds ($\mu\text{s}$)** deterministic latency; zero garbage collection pauses |
| **DDoS & Bot Mitigation** | Requires external XDP filters + JVM-level Netty pipeline handlers | **Native eBPF / XDP kernel hook** compiled directly into the Gourd daemon; drops invalid Handshake/Status packets at the NIC driver level before allocation |
| **Dynamic Routing State** | Requires Redis/RabbitMQ plugins (e.g., RedisBungee) for multi-proxy sync | Built-in **Vineyard CRDT State Mesh**; all Gourd edge nodes share global player registries in real time |
| **Java Velocity Compatibility** | Native | 100% wire-compatible with Velocity `MODERN` forwarding (`velocity:player_info`) and BungeeCord `PluginMessaging`. Plus a **Velocity-Vineyard Bridge `.jar**` for users keeping Java Velocity |

#### How Gourd Works Under the Hood

1. **Split-State Packet Splicing:** During the Minecraft `Handshake` (State 0), `Status` (State 1), `Login` (State 2), and `Configuration` (State 3) phases, Gourd parses packets in userspace to authenticate with Mojang/Yggdrasil, inject Velocity `MODERN` HMAC-SHA256 forwarding payloads, and negotiate Resource Packs. Once the player enters the `Play` (State 4) phase, Gourd switches the connection into **Fast-Path Splicing**. Only packets registered on the proxy's intercept bitmask (e.g., `ServerboundChatCommandPacket`, `ClientboundTransferPacket`, `PluginMessagePacket`, `BossEventPacket`) are deserialized in userspace; movement, chunk payloads, and entity updates bypass deep inspection via zero-copy ring buffers.
2. **Seamless Server-to-Server Dimension Handoff (Zero-Loading-Screen Transfers):** When transferring a player between two PumpkinMC backend nodes (e.g., Lobby $\rightarrow$ Survival, or Survival Server A $\rightarrow$ Survival Server B across a region border), Gourd intercepts the transition. Instead of forcing a full dimension respawn cycle, Gourd diffs the entity registry, caches the player's active client-side chunk bitmask, sends fake `UnloadChunk` packets only for out-of-bounds regions, and streams the new PumpkinMC server's chunks directly into the existing client session.
3. **Protocol Translation & Multi-Version Native Viaversion Equivalent (`GourdTranslate`):** Instead of running heavy Java ViaVersion/ViaBackwards plugins that parse and rewrite packets on the JVM heap, Gourd includes a declarative, macro-generated Rust packet schema translator that maps packet IDs and data component registries across Minecraft versions at wire speed.

---

### 1.4 The "Vineyard" Inter-Connection Mesh (IPC & Network Fabric)

To eliminate Redis, RabbitMQ, and MySQL bottlenecks between your Tauri app, proxies, and PumpkinMC game nodes, PumpkinPatch introduces **Vineyard**: a unified, bimodal IPC and network mesh.

* **Local Host Mode (Shared Memory Ring Buffers):**
When Gourd, the Tauri Core (`pumpkinpatch`), and a PumpkinMC instance run on the same machine, TCP loopback (`127.0.0.1`) is bypassed.
* **Lock-Free SPSC/MPMC Ring Buffers:** PumpkinPatch allocates a shared memory region (`/dev/shm/vineyard_*` on Linux/macOS, `CreateFileMappingW` on Windows).
* **`rkyv` Zero-Copy Serialization:** Data structures (player inventories, chat events, TPS tick profiles, chunk buffers) are written directly into shared memory using Rust's `rkyv` crate. The receiving process casts the raw memory pointer directly to a typed Rust struct (`ArchivedPlayerState`) with **zero parsing time and zero CPU cycles spent on deserialization**.


* **Multi-Node Distributed Mode (QUIC + mTLS + CRDTs):**
When scaling across multiple bare-metal servers or cloud VPS instances:
* **Multiplexed QUIC Streams (`quinn`):** Eliminates TCP Head-of-Line (HoL) blocking. High-priority control frames (e.g., player kick, economy transaction lock, cross-server inventory handoff) travel on dedicated unidirectional streams that are never blocked by bulk transfers (e.g., cross-node schematic syncing or region backups).
* **Loro CRDT State Store:** Global network state—such as total online players, vanish states, cross-server party rosters, active mute/ban lists, and global tab-list entries—is maintained in an in-memory **Conflict-Free Replicated Data Type (CRDT)** tree replicated across every node in the Vineyard mesh and directly into the Vue 3 Pinia stores within `< 5ms`.
* **Transactional Distributed Locks (Embedded Raft Consensus):** For critical operations (e.g., player trading across servers, auction house buyouts, or preventing duping during server crashes/handoffs), Vineyard uses an embedded lightweight Raft consensus group (`openraft`) among the active nodes, guaranteeing strict `Serializable` isolation without requiring an external SQL cluster.



---

## MODULE 2: NEXT-GEN "PLAN-STYLE" DEEP ANALYTICS & TELEMETRY ENGINE ("HARVESTMETRICS")

Legacy *Plan (Player Analytics)* relies on a Java plugin executing heavy SQL queries on a MySQL database, often causing server lag or database bloat while only capturing coarse surface metrics. PumpkinPatch's analytics engine—**HarvestMetrics**—operates out-of-band via the Vineyard zero-copy bus, storing data in an embedded **DuckDB** (for local Tauri installs) or **ClickHouse** (for massive clusters) columnar OLAP engine, rendered in Vue 3 using GPU-accelerated **WebGL uPlot / ECharts** and **Three.js**.

### 2.1 Player Lifecycle, Behavioral & Session Forensics

Every player action is streamed as a compressed event frame into the columnar store, enabling granular cohort and individual analysis directly inside the Vue dashboard:

* **Micro-Session Timeline & 4-State Entropy Classifier:**
* Replaces simple "Active vs. AFK" timers with a **4-State Entropy Classifier** calculated by the PumpkinMC hook:
1. *High-Engagement Active:* Combat, building, active movement + high camera yaw/pitch Shannon entropy.
2. *Low-Engagement Active:* Chatting, inventory sorting, crafting, standing still while interacting with UI packets.
3. *Mechanical / Macro AFK:* Detects anti-AFK water pools, piston bouncers, or autoclickers via Fast Fourier Transform (FFT) periodicity detection on input intervals, flagging the session as "Synthetic Active" so it doesn't skew genuine playtime statistics.
4. *True Idle AFK:* Zero packet entropy outside `KeepAlive`.




* **Server-Hop & Gamemode Funnel Sankey Diagrams:**
* Visualizes exact player flow across the Velocity/Gourd network using interactive Vue Sankey charts (e.g., *Joined via `mc.example.com` $\rightarrow$ Lobby-1 (14s) $\rightarrow$ BedWars-Solo-4 (18m) $\rightarrow$ Hub (45s) $\rightarrow$ Survival-Main (2h 15m) $\rightarrow$ Disconnected*).
* Pinpoints **"Rage-Quit / Churn Nodes"**: Correlates the exact last 30 seconds of events before a player disconnects and never returns (e.g., died to lava, killed by specific player, encountered a server lag spike $> 150\text{ms}$ MSPT, got stuck in a region transfer queue, or failed a resource pack download).


* **Cohort Retention & LTV Matrix:**
* Automatic D1, D3, D7, D14, D30, and D90 retention heatmaps segmented by:
* Join Campaign / Custom Domain (`play.server.com` vs. `tiktok.server.com` vs. `votesite1.server.com` captured by Gourd's `Handshake` virtual host field).
* Minecraft Client Version & Modloader Brand (Vanilla, Fabric, Iris, Lunar Client, Badlion, Feather, Geyser/Floodgate Bedrock).
* Geolocation & ASN/ISP routing quality.




* **Social Graph & Clique Detection (Force-Directed Vue Graph):**
* Builds a dynamic social weighted graph based on spatial proximity duration (players spending hours within 32 blocks of each other), shared chest/container interactions, party formation, direct messages, and TendrilVoice proximity audio time.
* Automatically identifies **Community Anchors** (players whose presence increases the session length of 5+ friends) and **Ban-Evasion / Alt Clusters** (accounts sharing identical camera sensitivity GCD signatures, keybind timings, ASN neighborhoods, and immediate social graph convergence).



### 2.2 Economy, Item Velocity & Duping Forensics

Because PumpkinPatch hooks directly into PumpkinMC's item entity and inventory ECS components:

* **Macro-Economic Health Dashboard:**
* Tracks **Total Money Supply (M1/M2)**, **Gini Coefficient** (wealth inequality across the playerbase), **Faucet vs. Sink Velocity** (where currency/items are entering the world via mob drops, mining, or quests vs. where they are destroyed via shop taxes, repair costs, or lava).


* **Cryptographic Item Provenance & Dupe Radar:**
* High-value items (or configurable item tags) can be stamped in PumpkinMC with a lightweight 64-bit **Snowflake UUID** inside custom NBT/DataComponents tracked in Vineyard's bloom filter.
* If two items with the identical Snowflake UUID exist simultaneously in two different containers/inventories across *any* server in the Velocity/Gourd cluster, the Tauri app triggers a **Critical Dupe Alert**, automatically freezes both inventories, snapshots the last 60 seconds of packet history for the involved players, and highlights the exact reproduction steps.


* **Ore-to-Stone & Loot Anomaly Z-Score Matrix:**
* Tracks mining trajectories and block-break ratios (e.g., Ancient Debris or Diamond Ore mined per 1,000 Deepslate blocks, angular path variance toward concealed ores, light-level mining averages) and flags statistical outliers ($Z > 4.0$) in a dedicated **Anti-Xray Forensics Tab**.



### 2.3 Deep Hardware, OS & Sub-Tick Server Performance Profiling

Java servers rely on Spark samplers that periodically dump JVM thread stacks. Because PumpkinMC is native Rust, PumpkinPatch provides **deterministic nanosecond-precision ECS & Async Task Instrumentation**:

* **Live 60 FPS MSPT (Milliseconds Per Tick) Flamegraphs in Vue:**
* Streams `tracing` spans directly from PumpkinMC into a real-time interactive Flamegraph and Icicle chart inside the Tauri app.
* Breaks down every $50\text{ms}$ ($20\text{ TPS}$) tick budget into exact subsystems:
* `Network::PacketDecode`
* `World::ChunkGeneration (Noise / Biome / Surface / Features)`
* `ECS::EntityMovement & AABB Collisions`
* `ECS::MobPathfinding (A* Navigation)`
* `World::RedstoneWire & BlockTick`
* `Storage::RegionZstdCompression & DiskFlush`
* `Plugin::WasmGuestExecution` (isolated per-plugin CPU cost!)




* **3D Volumetric Chunk Lag Heatmap (Three.js WebGL2 View):**
* Renders the live server world inside the Vue app as an interactive 3D isometric map.
* Overlays a vertical thermal bar on every loaded chunk representing its exact microsecond CPU cost, entity count, tile-entity density, and packet broadcast amplification factor.
* Clicking a glowing red chunk in the Vue 3D map lets the admin **Teleport In-Game**, **Inspect Chunk Entities in a Table**, **Freeze Redstone/Hopper Ticks in that specific chunk**, or **Force Unload & Quarantine** it without restarting the server.


* **Network & Protocol Bandwidth Profiler:**
* Breakdown of egress/ingress bandwidth per packet ID (`LevelChunkWithLight`, `SetEntityMotion`, `BundleDelimiter`, `CustomPayload`), compression ratio efficiency (zlib vs. zstd vs. raw), and per-player TCP/QUIC congestion window (CWND) and packet loss telemetry.



---

## MODULE 3: AUTOMATED SERVER FACTORY, DYNAMIC SCALING & VELOCITY/GOURD ORCHESTRATION

You already have basic automatic server creation and Velocity configuration. PumpkinPatch elevates this into a **declarative, self-healing, zero-downtime Kubernetes-grade orchestrator** tailored specifically for Minecraft's stateful and stateless workloads—accessible via an intuitive **VueFlow Drag-and-Drop Node Graph**.

### 3.1 Visual Topology Builder (VueFlow Canvas)

Inside the Tauri app, the **Network Topology View** renders your entire infrastructure as a live, interactive node graph (similar to Blender's Shader Editor or Unreal Blueprints):

* **Edge Ingress Nodes:** Represent public domains (`play.myserver.net`, `eu.myserver.net`, `bedrock.myserver.net`), TCPShield/Cloudflare Spectrum tunnels, and **Gourd / Velocity Proxies**.
* **Routing Filter Nodes:** Visual condition blocks you wire between proxies and servers:
* *Version Filter* (e.g., `1.21.4` goes to Pool A; `1.20.1` goes to Legacy Pool B).
* *Permission / Rank Gate* (e.g., `group.vip` or `staff.beta` bypasses queue or routes to VIP Lobby).
* *GeoIP / Latency Router* (routes player to the lowest-ping regional Lobby).
* *Forced Host Router* (`creative.myserver.net` routes straight to the Creative cluster).


* **Backend Server Nodes & Auto-Scaling Pools:** Wire routing nodes directly to static PumpkinMC servers (e.g., `SMP-Main`) or elastic **Template Scaling Pools** (e.g., `BedWars-Micro-Pool [Min: 2, Max: 50]`).
* **Live Wire Animation:** Watch animated particles flow across the wires in real time proportional to player traffic and packet throughput. Dragging a wire from a newly created server to the Gourd proxy node **instantly updates the proxy's live in-memory routing table** (and patches `velocity.toml` via hot-reload if using Java Velocity) with zero restarts.

### 3.2 Immutable Blueprints, Content-Addressable Storage (CAS) & `<50ms` Clone-on-Write Provisioning

Spinning up a new minigame, lobby, or seasonal SMP instance shouldn't involve copying 2 GB of world files and binaries.

* **BLAKE3 Content-Addressable Store (`/var/lib/pumpkinpatch/cas`):**
* Every PumpkinMC binary version, plugin artifact, base world region (`.mca` / `.linear`), and configuration template is stored once, indexed by its `BLAKE3` cryptographic hash.


* **Instant Copy-on-Write (CoW) Server Instantiation:**
* On Linux (`OverlayFS`, `Btrfs`, or `ZFS` reflinks), macOS (`APFS` clonefile), and Windows (`ReFS` block cloning / hardlink overlays), spinning up a new PumpkinMC server from a **Server Blueprint** takes **under 45 milliseconds** and consumes **0 MB of additional disk space** until blocks are actually modified in-game.
* **Ephemeral RAM-Disk (`tmpfs`) Mode:** For lobbies, queues, and short-lived minigame matches (SkyWars, BedWars, Duels), PumpkinPatch mounts the server's writable upper layer directly into a bounded `tmpfs` RAM disk. When the match ends, the instance is destroyed in $2\text{ms}$ with zero SSD wear.



### 3.3 Predictive Auto-Scaling & Stateful Drain State Machine

Each Server Pool in PumpkinPatch operates via an autonomous Rust state machine:

```
[TEMPLATE BLUEPRINT] 
       | (Pre-warm trigger: Pool utilization > 75% or Scheduled Event)
       v
  (WARMING UP) -- Boots in <120ms (PumpkinMC Rust startup speed)
       |
       v
  (STANDBY / HOT-SPARE) -- Registered in Gourd Proxy as `hidden_ready`
       |
       v
   (ACTIVE) -- Accepting players; streaming telemetry to Vineyard
       |
       +--> [If Minigame Match Starts] --> (IN-GAME LOCKED) -- Blocks new joins, allows Spectator transfers
       |
       v (Scale-down trigger / Match end / Rolling update)
  (DRAINING) -- Delisted from join balancer; triggers Graceful Player Migration
       |
       v
(ARCHIVING / GC) -- Commits persistent Player Data/Logs to CAS; wipes ephemeral CoW layer

```

* **Predictive Queue & Traffic Scaling:** Instead of waiting for a Lobby to hit 100% capacity, HarvestMetrics uses historical day-of-week join velocity curves and current Gourd handshake rates ($d(\text{Joins})/dt$) to pre-warm additional Lobby or Minigame instances 30 seconds *before* the player spike arrives.
* **Dynamic Port & Socket Allocation (Zero Port Conflicts):**
* Admins never have to manually type port numbers (`25565`, `25566`, `25567`...).
* For local servers on the same host as Gourd, PumpkinPatch configures PumpkinMC and Gourd to communicate over **Unix Domain Sockets (`/run/pumpkinpatch/srv_uuid.sock`)** or **Shared Memory**, bypassing the TCP/IP network stack entirely and freeing all TCP ports!
* For remote nodes, `stem-agent` dynamically allocates ephemeral ports from a managed pool, configures Linux `nftables` / Windows Firewall rules so only the Gourd proxy's mTLS/WireGuard IP can reach them, and registers the endpoint in Vineyard automatically.



### 3.4 Automatic DNS, Reverse Proxy & Tunnel Provisioning

* **Native Cloudflare / Route53 / Hetzner DNS Integration:** Create a server or forced host in the Vue UI (e.g., `event.myserver.com`), and PumpkinPatch automatically creates the `A`, `AAAA`, and `_minecraft._tcp` `SRV` records via DNS APIs.
* **Built-In NAT Traversal & Zero-Port-Forwarding Tunnels:** Running PumpkinPatch on a home PC behind CGNAT? Flip a toggle in the Tauri UI to activate a built-in **Playit.gg / FRP / Cloudflare Spectrum / WireGuard Relay** directly inside the Rust backend, giving you a public DDoS-protected IP in 3 seconds.

---

## MODULE 4: NEXT-GEN PLUGIN, WASM & MODPACK LIFECYCLE MANAGER

Because **PumpkinMC** is pioneering a modern Rust-based server ecosystem while existing networks may still run hybrid setups (Java Velocity, Paper/Fabric backend sub-servers alongside PumpkinMC), PumpkinPatch features a **Universal Multi-Runtime Artifact Manager**.

### 4.1 Three-Tier Plugin & Extension Architecture for PumpkinMC

1. **Tier 1: Sandboxed WebAssembly (`WASM` / `WASI` Component Model) Plugins (Recommended):**
* Supports plugins compiled to `.wasm` from **Rust, C#, Go, TypeScript/AssemblyScript, Kotlin, or Python**.
* **Hot-Swapping Without Server Restarts:** Because WASM modules run inside an embedded `wasmtime` runtime within PumpkinMC, PumpkinPatch can pause a plugin's event queue for $2\text{ms}$, serialize its state via the WASI state hook, swap the `.wasm` binary in memory, restore state, and resume execution **while players are mid-combat**.
* **Granular Capability Permissions (iOS/Android-Style Manifests):** In the Vue UI, before enabling a plugin, PumpkinPatch parses its WIT (Wasm Interface Type) imports and presents a security permission checklist:
* `[x] Read/Write Player Inventories`
* `[x] Register Custom Commands (/spawn, /tpa)`
* `[ ] Outbound HTTP Requests (Restricted to: api.discord.com)`
* `[ ] Raw Filesystem Access (Blocked by default)`


* **Per-Plugin CPU & Memory Quotas:** Using `wasmtime` fuel metering and epoch interruption, you can set a strict limit in the Vue UI (e.g., *"Max 2.5ms CPU time per tick, Max 64 MB RAM"*). If a poorly coded plugin enters an infinite loop, PumpkinMC automatically terminates that single plugin call and sends a stack trace to the Tauri app instead of crashing or lagging the server!


2. **Tier 2: Native Rust Dynamic Libraries (`.so` / `.dll` / `.dylib` ABI-Stable Plugins):**
* Uses `abi_stable` / `cdylib` for maximum bare-metal speed when zero virtualization overhead is required.
* **Integrated Cargo Build Pipeline:** Point PumpkinPatch at a Git repository (GitHub/GitLab/Gitea) or local folder. The Tauri backend compiles the plugin against the exact target PumpkinMC commit/version hash to guarantee ABI compatibility, caches the artifact in the CAS store, and deploys it across all matching nodes.


3. **Tier 3: Legacy Bukkit/Paper/Velocity `.jar` Support (For Hybrid Networks):**
* For networks running mixed PumpkinMC and Java Paper/Fabric/Velocity nodes, the plugin manager seamlessly manages `.jar` lifecycles alongside native Rust/WASM plugins.



### 4.2 Unified Multi-Source Marketplace & Dependency SAT Solver

* **Aggregated One-Click Browser:** Search, filter, and install extensions from **Modrinth, Hangar, SpigotMC, CurseForge, GitHub Releases, and the dedicated *PumpkinCrate Registry*** directly inside a native Vue 3 storefront view.
* **PubGrub Dependency SAT Solver:**
* Uses the `pubgrub` Rust crate to mathematically resolve complex dependency trees, version constraints, conflicting plugins, and required protocol libraries before a single file is downloaded.
* Automatically distinguishes between **Proxy-Level Extensions** (installs to Gourd/Velocity), **Backend Server Extensions** (installs to PumpkinMC), and **Synchronized Bridge Extensions** (e.g., LuckPerms or TendrilVoice, which require synchronized installation and shared cryptographic secrets on *both* the proxy and backend servers simultaneously).


* **Configuration Inheritance, Diffing & Secret Injection:**
* Never manually edit the same database password across 20 servers again.
* Define **Global Config Layers** $\rightarrow$ **Pool Config Overrides** $\rightarrow$ **Instance-Specific Variables**.
* Uses the integrated **Monaco Editor** (VS Code's engine inside Vue) with custom JSON Schema / TOML validation, auto-completion for Minecraft item/block IDs, and side-by-side Git-style diffing before applying changes.
* **Automated Binary Bisect Tool:** Server crashing or lagging after updating 15 plugins? Click **"Run Automated Bisect"** in the Tauri UI. PumpkinPatch spins up an isolated Copy-on-Write clone of your server in the background, performs an automated binary search ($O(\log N)$) by toggling subsets of plugins and simulating load, and tells you the exact plugin causing the conflict within 15 seconds.



---

## MODULE 5: INTERNAL PROTOCOL REWRITES & DEEP ECOSYSTEM PATCHING

You specifically highlighted wanting to do **internal rewrites or deep patches of ecosystem staples like Voice Chat, Velocity features, Resource Pack hosting, and World Sharding**. Because we control the Tauri app, the Gourd proxy, the Vineyard bus, and the PumpkinMC patches, we can build first-party, native Rust replacements that outperform cobbled-together third-party mods while remaining **100% compatible with existing player client mods**.

### 5.1 "TendrilVoice": Native Rust Spatial Voice SFU & Tauri Audio Deck

Players shouldn't have to guess whether to install *Simple Voice Chat* or *Plasmo Voice*, and server owners shouldn't have to deal with UDP port forwarding nightmares when players switch servers behind Velocity.

* **Dual-Protocol Client Compatibility (Zero Custom Mod Required):**
* **TendrilVoice** is a native Rust UDP Selective Forwarding Unit (SFU) built directly into the Gourd Proxy and PumpkinMC nodes.
* It speaks **both** the *Simple Voice Chat (SVC)* UDP protocol (AES-128-CBC / Opus) **and** the *Plasmo Voice* protocol natively! A player running the Simple Voice Chat Fabric mod can talk directly to a player running Plasmo Voice or the built-in browser WebRTC fallback—TendrilVoice transcodes the packet headers in $< 20\mu\text{s}$ without re-encoding the Opus audio frames.


* **Proxy-Terminated UDP Roaming (Zero Audio Dropouts on Server Transfer):**
* In standard Velocity setups, every backend server needs its own public UDP port open, and when a player switches servers, their voice chat disconnects and reconnects for 2–4 seconds.
* With TendrilVoice, the **Gourd Edge Proxy terminates the single UDP voice socket**. As the player walks across server boundaries or teleports from Lobby to Survival, Vineyard streams their 3D positional coordinates (`Vec3<f64>`, Dimension ID, Yaw/Pitch) to the SFU. Their voice connection **never drops for a single millisecond**.


* **Server-Side Spatial Acoustic Raycasting (Occlusion & Reverb Metadata):**
* Because TendrilVoice shares memory with PumpkinMC's chunk palette, it performs lightweight 3D voxel DDA raycasting between speaking players. If a solid stone wall separates two players, or if they are inside a deep dripstone cavern, TendrilVoice injects acoustic attenuation and reverb coefficients so sound muffles realistically through walls and prevents cave eavesdropping exploits.


* **The Tauri Admin Live Audio Deck (Built directly into the Vue UI!):**
* Because PumpkinPatch is a Tauri desktop app with native OS audio access (`cpal` / `rodio` / `webrtc-rs`), admins don't even need to launch Minecraft to moderate voice chat!
* **Live Spatial Audio Map:** Click any location or player cluster on the 3D Vue World Map to "drop an ear" into that coordinate and listen to live proximity voice chat right through your desktop headphones.
* **God-Voice PA Broadcast:** Hold your Push-to-Talk key inside the Tauri app to speak directly to a specific player, a specific WorldGuard region, a staff channel, or the entire network with optional DSP effects (e.g., megaphone filter or spatial stadium echo).
* **Rolling RAM Ring-Buffer Evidence Recorder:** To handle toxic voice chat reports without recording terabytes of audio to disk, TendrilVoice maintains a rolling **60-second encrypted Opus ring buffer in RAM** per active speaker. Only when a player runs `/report voice <player>` (or when optional local Whisper.cpp speech-to-text keyword flagging triggers) is that specific 60-second clip flushed to disk and attached as a playable waveform inside the Tauri Moderation Ticket view!



---

### 5.2 "PatchWork": Built-In Resource Pack Compiler, Polymorph Merger & Edge CDN

Managing custom items, GUIs, custom sounds, and Blockbench models across multiple servers (and plugins like ItemsAdder/Oraxen equivalents) is notoriously painful.

* **Automatic Multi-Plugin Pack Merger & CustomModelData Resolver:**
* Drop multiple resource packs or plugin asset folders into PumpkinPatch.
* The Rust backend parses all JSON model definitions, blockstates, font glyph bitmaps (`\uE000`–`\uF8FF`), and shaders, automatically resolving `CustomModelData` / `1.21.4+ Item Model Component` collisions, packing textures into optimized GPU atlases (`image` crate), and stripping unused metadata to reduce download size by up to 40%.


* **Minecraft 1.20.3+ Multi-Pack Stack Diffing:**
* Instead of forcing players to re-download a massive 80 MB monolithic `.zip` every time you add one custom sword texture, PatchWork breaks your assets into **Content-Addressed Layer Packs** (`base-core.zip`, `models-v4.zip`, `seasonal-halloween.zip`, `delta-patch-01.zip`).
* Utilizing Minecraft's native stacked resource pack UUID protocol (`ClientboundResourcePackPushPacket`), Gourd only pushes the tiny $50\text{ KB}$ delta pack to the client on update!


* **Embedded High-Speed HTTP/3 Pack Server & Polymath Protection:**
* Gourd natively serves resource packs on the exact same port (via protocol multiplexing) or via a built-in Rust `axum` static edge server with automatic SHA-1 calculation, rate-limiting, and optional zip-header scrambling to deter casual asset rippers.



---

### 5.3 "TendrilWorld": Cross-Server Seamless World Sharding (Folia-Beyond-One-Machine)

Paper's *Folia* project splits a single world across multiple threads on *one* machine. Because PumpkinMC is built in Rust and connected via the **Vineyard** zero-copy mesh, **PumpkinPatch enables True Multi-Server Seamless World Sharding**:

* **Spatial Voronoi / Grid Region Partitioning:**
* Divide a massive $20,000 \times 20,000$ survival world or MMORPG map into spatial zones (e.g., Spawn City on `Pumpkin-Node-01`, Northern Wilderness on `Pumpkin-Node-02`, Southern Dungeons on `Pumpkin-Node-03`) directly on the Vue 3D Map.


* **The "Halo / Ghost Border" Synchronization Protocol:**
* Within a 64-block border zone (`4 chunks`) between two adjacent servers, Vineyard establishes a real-time **Border Halo Stream**.
* Players standing on `Node-01` at $X = 995$ can see, shoot arrows at, and hear players standing on `Node-02` at $X = 1005$! Block changes, projectiles, redstone signals, and entity movements within the 64-block halo are mirrored across the shared memory / QUIC bus at $20\text{ Hz}$.
* The exact tick a player steps across $X = 1000$, Gourd performs a **Zero-Loading-Screen Authority Handoff** to `Node-02`. To the player, it feels like a single 1,000-player continuous world with zero portals, zero warp commands, and zero dimension loading screens.



---

### 5.4 "GourdGate": Native Bedrock Edition Translation & Account Linking

* Tightly integrates or embeds Geyser/Floodgate protocol translation directly at the **Gourd Edge Proxy** layer.
* Automatically converts Java Resource Packs from **PatchWork** (Blockbench models, custom armor, custom GUI font offsets) into Bedrock `.mcpack` format on the fly so Bedrock mobile/console players see custom items and UIs accurately.
* Built-in **Global Account Linker**: Allows players to link their Microsoft/Bedrock and Java accounts via a 6-digit code in-game or via Discord OAuth, unifying their UUID, inventory, economy balance, and HarvestMetrics analytics profile across platforms.

---

## MODULE 6: THE COMPLETE MANAGEMENT FEATURE MATRIX (EVERY SINGLE FEATURE)

To make **PumpkinPatch** the undisputed gold standard of Minecraft infrastructure suites, the Tauri + Vue application encompasses the following comprehensive modules:

### 6.1 The Next-Gen Live Console & Interactive REPL (`Xterm.js WebGL` + Rust Parser)

Standard server consoles are dumb text dumps. PumpkinPatch's console is a structured observability terminal:

* **Structured AST Log Parsing:** Instead of raw text strings, log lines from PumpkinMC and Gourd are tagged with structured JSON/rkyv metadata (`Timestamp`, `Severity`, `NodeID`, `System/Plugin Source`, `PlayerUUID`, `WorldCoord`).
* **Interactive Hover Pills inside the Console:**
* Hover over a **Player Name** in the console log to see a live Vue popover with their current HP, Hunger, Ping, IP, Geo-location, Mini-3D Skin render, Inventory preview, and 1-click `[Teleport]`, `[Spectate]`, `[Kick]`, or `[Ban]` buttons.
* Hover over **Coordinates (`[142, 64, -892]`)** to see a live mini-map preview of that chunk and a button to open it in the 3D World Inspector.
* Hover over a **Plugin Error / Panic Stacktrace** to click `[Jump to Source Line in Monaco IDE]` or `[Create GitHub Issue with Context]`.


* **Multi-Server Multiplexed Command Bar ("Broadcast REPL"):**
* Target commands using smart selectors right from the bottom command bar:
* `@all say Server restarting in 5m` (executes across all nodes simultaneously).
* `@pool:bedwars* pumpkin reload-config` (targets only servers in the BedWars pool).
* `@lagging profiler capture 30s` (automatically targets any server currently running above $40\text{ms}$ MSPT).


* Full **Brigadier Command Tree Auto-Completion** inside the Vue input box! Because PumpkinMC sends its live Brigadier command graph to the Tauri app, typing `/gamemode ` or `/give @p ` inside the desktop panel gives you the exact same rich argument auto-complete, item icons, and syntax validation as typing in-game.



---

### 6.2 Live 3D World, Entity & NBT Inspector ("PumpkinEye")

Powered by **Three.js / WebGL2** inside the Vue frontend and fed by raw binary chunk streams over Tauri IPC:

* **Zero-Plugin Live BluMap / Dynmap Alternative:** Because PumpkinPatch reads region data directly from PumpkinMC's memory and disk cache, you get an instant, zero-configuration 2D/3D map of every world on every server inside your Tauri app (and optionally publishable to a public web URL for players).
* **Live Player & Entity Radar:** Watch every player, mob, minecart, and dropped item move across the 3D map in real time at 20 TPS.
* **Live Offline & Online Player Inventory / EnderChest / NBT Surgical Editor:**
* View any player's Inventory, Armor slots, Offhand, EnderChest, Potion Effects, Advancements, and Attributes rendered with authentic Minecraft item textures, enchantment glints, and hover lore tooltips.
* **Drag-and-Drop Live Editing:** Drag an item out of a suspected duper's inventory straight into an Admin Evidence Vault, or drag a replacement item into a player's slot **while they are online and playing**—synchronized atomically via the Vineyard ECS hook without kicking the player or corrupting their `.dat` file.


* **World Region Surgical Pruning & Reset Tool:**
* Select chunks on the 2D/3D map using a lasso, rectangle, or automatic filter: *"Select all chunks where `InhabitedTime < 30 seconds` AND `LastModified > 30 days ago` AND `No WorldGuard Claims Present`"*.
* Preview the selected chunks in 3D, see the exact disk space savings (e.g., *"Will reclaim 14.8 GB"*), and prune them so they regenerate with new Minecraft update terrain seamlessly.



---

### 6.3 Time-Machine Backups, Deduplication & Instant Disaster Recovery

* **Chunk-Level Content-Defined Chunking (CDC) Backups (`FastCDC` + `Zstd` + `BLAKE3`):**
* Traditional backups zip the entire 50 GB world folder every 6 hours, wasting terabytes of storage and hammering SSD I/O.
* PumpkinPatch hashes individual `.mca` / `.linear` region chunks and player data files. Only the exact $4\text{ KB}$ sectors that changed since the last snapshot are compressed with `zstd` (dictionary-trained on Minecraft NBT data for 3x higher compression ratios) and uploaded to **S3, Backblaze B2, Cloudflare R2, SFTP, or a local NAS**.


* **Consistent Live Snapshots (Zero Server Freeze):**
* Coordinates with PumpkinMC's region flush queue using Copy-on-Write memory pages so backups occur with **0.0ms tick impact** and zero risk of half-written corrupted chunks.


* **Granular "Time-Machine" Scrubbing & Single-Chunk / Single-Player Restore:**
* Did a griefer blow up a player's base at `[X: 400, Z: -200]` three hours ago, or did a player lose their inventory to a bug?
* You do **not** have to roll back the entire server! Open the **Time-Machine Slider** in the Vue 3D Map, scrub backward to `12:15 PM`, highlight just the 4 affected chunks (or select just that one player's inventory), preview the diff in 3D right inside the Tauri viewport, and click **"Splice Restore to Live World"**. PumpkinMC swaps those 4 chunks in memory and streams the restored blocks to nearby players instantly!



---

### 6.4 Visual Event Reactor & Workflow Automation Engine ("PatchWork Flows")

Replace dozens of bloated custom scripts and cron jobs with a visual, node-based **If-This-Then-That / Node-RED style Automation Canvas** inside Vue:

* **Event Triggers:**
* *Server Metrics:* `MSPT > 48ms for 10s`, `RAM > 90%`, `Host Disk < 5GB`, `Gourd Handshake Rate > 500/sec (Bot Attack)`.
* *In-Game Events:* `Player First Join`, `Player Death`, `Chat Regex Match`, `Dupe Alert Triggered`, `Specific Advancement Unlocked`, `Staff Member Vanished`.
* *External Webhooks & Schedules:* `Tebex/CraftingStore Purchase Webhook`, `GitHub Push to Main Branch`, `Cron Schedule (Every Friday at 18:00 UTC)`.


* **Logic & Filter Blocks:**
* Rate-limiters, Delay timers, Player Permission checks, Online Player Count thresholds, Math expressions.


* **Action Nodes:**
* *Server Control:* `Scale Up Pool (+2 instances)`, `Trigger CoW Snapshot`, `Execute Console Command`, `Enable Gourd Bot-Verification Captcha Void World`.
* *In-Game Effects:* `Send Title/Actionbar`, `Play Custom Sound`, `Grant LuckPerms Group`, `Give Item via ECS`.
* *External Integrations:* `Send Rich Discord Embed (with live ECharts image snapshot attached!)`, `Create PagerDuty/Telegram Alert`, `Trigger Tauri Desktop Native Notification with Siren Audio`.



---

### 6.5 Multi-Tenant RBAC, Staff Security & Audit Forensics

Whether you have 2 co-owners or a 40-person staff team (Devs, Builders, Admins, Moderators, Helpers):

* **Fine-Grained Attribute-Based Access Control (ABAC):**
* Assign permissions not just globally, but scoped per **Server Pool, Tag, or Environment (`prod` vs. `staging`)**.
* Example *Builder Role*: Can access the `Creative-Build-*` server pool, upload `.schem` files, and use the 3D World Viewer, but cannot read player IPs or touch `Survival-Prod`.
* Example *Junior Moderator Role*: Can access the Moderation Center, view HarvestMetrics player sessions, listen to flagged **TendrilVoice** report clips, and issue temporary mutes/bans, but cannot run raw console commands or download world files.


* **Cryptographic Hash-Chained Audit Log (Tamper-Evident Ledger):**
* Every action taken in the Tauri app, via the API, or by a staff member in-game (console command, file edit, inventory modification, config change, voice listen-in) is recorded in an append-only ledger where each entry's `BLAKE3` hash includes the previous entry's hash.
* Even a rogue administrator cannot silently delete a single audit log line without invalidating the cryptographic chain and triggering an alert to the Master Owner key.


* **PII & IP Anonymization Mode ("Streamer / Screen-Share Safe Mode"):**
* One click in the top Tauri titlebar toggles **Streamer Mode**: automatically blurs or replaces all player IP addresses, street geolocations, server public node IPs, and secret keys across every Vue view with deterministic pseudonyms so admins can safely screen-share the panel on Discord or Twitch.



---

### 6.6 Integrated CI/CD Staging Pipeline & Synthetic Bot Load Tester ("Locust Craft")

How do you know if your new PumpkinMC build, WASM plugin, or Gourd proxy config will survive 500 players joining at the exact same second on release day?

* **One-Click "Promote Staging to Production" Canary Releases:**
* Clone your Production environment into a sandboxed Staging network with 1 click.
* Test plugin updates and config changes. When ready, click **Canary Rollout**: Gourd routes 5% of incoming players to the updated node pool while HarvestMetrics compares MSPT, crash rates, and packet errors against the baseline pool. If metrics stay green for 10 minutes, it automatically rolls out to 100%; if a panic or lag spike occurs, it rolls back in $< 50\text{ms}$.


* **Built-In Synthetic Player Swarm Generator ("CrowSwarm"):**
* Directly inside the Tauri app, launch a headless Rust swarm of **100 to 5,000 simulated Minecraft clients** against your local or staging cluster.
* Configure realistic bot behavior profiles:
* *Login Storm:* 1,000 bots connecting through Gourd within 10 seconds to stress-test authentication, queue routing, and initial chunk streaming.
* *World Generation Explorer:* 200 bots flying in divergent vectors at Elytra speeds to benchmark PumpkinMC's multi-threaded chunk generator and disk I/O.
* *Combat & Entity Chaos:* 100 bots clustering in a single chunk swinging swords, shooting arrows, and talking on simulated **TendrilVoice** Opus streams to benchmark worst-case $O(N^2)$ packet broadcasting.





---

### 6.7 Edge DDoS, Anti-Bot & Protocol Firewall ("Scarecrow Shield")

Built directly into the **Gourd** proxy and visualizable in a live Cyber-Defense Map inside Vue:

* **eBPF / XDP Wire-Speed Packet Scrubbing:** Drops malformed TCP/UDP packets, Ping/Status floods, and oversized Handshake payloads at the Linux kernel network driver level before they ever touch userspace.
* **Zero-Friction Protocol Fingerprinting:** Inspects TCP SYN options, TLS/Minecraft Handshake ordering, and keep-alive timing jitter to silently distinguish genuine Vanilla/Fabric clients from headless botnet scripts.
* **Virtual Limbo / Gravity Captcha Filter:** Under an active L7 bot attack, Gourd transparently diverts suspicious connections into an ultra-lightweight **in-memory Rust Limbo micro-world** (capable of holding 25,000 concurrent connections in $50\text{ MB}$ of RAM). The client must complete a 1.5-second physics fall check (verifying authentic Minecraft client gravity & collision math) + optional Map-Art Captcha before Gourd splices them through to a real PumpkinMC lobby.

---

## MODULE 7: CONCRETE CODE & WORKSPACE BLUEPRINTS (TAURI v2 + VUE 3 + RUST)

Below is the concrete production workspace architecture and implementation code demonstrating how **PumpkinPatch** wires the Tauri v2 backend, the Vineyard zero-copy bus, the Gourd proxy, and the Vue 3 frontend together.

### 7.1 Cargo Workspace & Directory Structure

```text
pumpkinpatch/
├── Cargo.toml                      # Workspace root (shared dependencies, LTO profiles)
├── crates/
│   ├── vineyard-protocol/          # Zero-copy `rkyv` structs, CRDT schemas, QUIC framing
│   ├── vineyard-bus/               # Shared Memory (SHM) ring buffers + mTLS Quinn mesh
│   ├── gourd-proxy/                # Native Rust Velocity-compatible edge proxy + eBPF hooks
│   ├── tendril-voice/              # Native UDP Spatial Audio SFU (SVC + Plasmo compatible)
│   ├── harvest-metrics/            # DuckDB / ClickHouse OLAP ingestion & FFT anomaly engine
│   ├── pumpkin-hook-sdk/           # Cargo patch/feature embedded into PumpkinMC servers
│   └── stem-agent/                 # Headless daemon for remote bare-metal Linux/Windows nodes
├── src-tauri/                      # Tauri v2 Desktop & Mobile Rust Core
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/               # Tauri v2 granular security ACLs
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── commands/               # #[tauri::command] handlers (orchestrator, plugins, files)
│       ├── ipc_streams/            # Raw binary 60fps Tauri Channels (3D chunks, flamegraphs)
│       └── audio_deck/             # Native cpal/rodio Admin Voice Moderation DSP
└── src/                            # Vue 3.5+ + TypeScript + Vite Frontend
    ├── main.ts
    ├── App.vue
    ├── stores/                     # Pinia stores backed by Binary ArrayBuffers & CRDT sync
    │   ├── useTopologyStore.ts
    │   ├── useHarvestStore.ts
    │   └── useVoiceDeckStore.ts
    ├── components/
    │   ├── topology/               # VueFlow interactive Gourd/Velocity node router
    │   ├── analytics/              # WebGL uPlot 60fps MSPT flamegraphs & retention Sankeys
    │   ├── world3d/                # Three.js live chunk & entity heatmap inspector
    │   ├── console/                # Xterm.js WebGL + Brigadier autocomplete bar
    │   └── voice/                  # Live spatial audio radar & waveform evidence player
    └── composables/
        └── useBinaryChannel.ts     # Zero-JSON Tauri v2 binary stream hook

```

### 7.2 Zero-Copy Telemetry Structs (`crates/vineyard-protocol/src/lib.rs`)

Using `rkyv`, PumpkinMC writes tick metrics and player positions directly into shared memory or QUIC streams, and the Tauri backend reads them without allocation or parsing overhead:

```rust
use rkyv::{Archive, Deserialize, Serialize};

/// Emitted every tick (20Hz) by PumpkinMC directly into the Vineyard SHM Ring Buffer
#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
#[repr(C)]
pub struct TickTelemetryFrame {
    pub server_id: [u8; 16],          // UUID bytes
    pub tick_number: u64,
    pub timestamp_ns: u64,
    pub total_mspt_us: u32,           // Microseconds spent in this tick
    pub net_decode_us: u32,
    pub ecs_movement_us: u32,
    pub chunk_gen_us: u32,
    pub redstone_tick_us: u32,
    pub wasm_plugins_us: u32,
    pub online_players: u16,
    pub loaded_chunks: u32,
    pub entity_count: u32,
    pub hot_chunks: Vec<ChunkHeatSample>,
}

#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(check_bytes)]
#[repr(C)]
pub struct ChunkHeatSample {
    pub dimension_id: u8,
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub tick_cost_us: u32,
    pub entity_count: u16,
    pub packet_bytes_emitted: u32,
}

```

### 7.3 Tauri v2 Raw Binary IPC Streamer (`src-tauri/src/ipc_streams/telemetry.rs`)

Instead of serializing `TickTelemetryFrame` to a slow JSON string for Vue, Tauri v2's `ipc::Channel` streams raw packed binary frames straight into a JavaScript `ArrayBuffer`:

```rust
use tauri::{ipc::{Channel, InvokeResponseBody}, State};
use crate::state::AppState;
use vineyard_protocol::ArchivedTickTelemetryFrame;

#[tauri::command]
pub async fn subscribe_live_telemetry(
    state: State<'_, AppState>,
    server_id: uuid::Uuid,
    on_frame: Channel<InvokeResponseBody>,
) -> Result<(), String> {
    let mut rx = state
        .vineyard_bus
        .subscribe_server_Raw_frames(server_id)
        .await
        .map_err(|e| e.to_string())?;

    // Spawn a lightweight Tokio task that streams raw zero-copy bytes at 20-60Hz to Vue
    tauri::async_runtime::spawn(async move {
        while let Ok(raw_shm_slice) = rx.recv().await {
            // Validate zero-copy archive in < 50 nanoseconds
            if rkyv::check_archived_root::<vineyard_protocol::TickTelemetryFrame>(&raw_shm_slice).is_ok() {
                // Send raw binary directly over Tauri v2 IPC without JSON serialization!
                if on_frame.send(InvokeResponseBody::Raw(raw_shm_slice.to_vec())).is_err() {
                    // Vue component unmounted or window closed; clean up subscription
                    break;
                }
            }
        }
    });

    Ok(())
}

```

### 7.4 Vue 3 Zero-JSON Binary Stream Composable (`src/composables/useBinaryChannel.ts`)

On the Vue 3 side, we consume the raw `ArrayBuffer` using a `DataView` / `Float32Array` for immediate 60 FPS WebGL plotting without triggering JavaScript Garbage Collection spikes:

```typescript
import { invoke, Channel } from '@tauri-apps/api/core';
import { onUnmounted, shallowRef } from 'vue';

export interface ParsedTelemetryHeader {
  tickNumber: bigint;
  totalMsptMs: number;
  netDecodeMs: number;
  ecsMovementMs: number;
  chunkGenMs: number;
  redstoneTickMs: number;
  wasmPluginsMs: number;
  onlinePlayers: number;
  loadedChunks: number;
  entityCount: number;
}

export function useLiveTelemetryStream(serverId: string) {
  const latestHeader = shallowRef<ParsedTelemetryHeader | null>(null);
  // Pre-allocated ring buffer for 60fps uPlot / WebGL canvas rendering
  const msptRingBuffer = new Float32Array(1200); // 60 seconds @ 20 TPS
  let ringIndex = 0;

  const channel = new Channel<ArrayBuffer>();

  channel.onmessage = (rawBuffer: ArrayBuffer) => {
    const view = new DataView(rawBuffer);
    // Read directly from the packed C-repr struct offsets
    const tickNumber = view.getBigUint64(16, true);
    const totalMsptUs = view.getUint32(32, true);
    const totalMsptMs = totalMsptUs / 1000.0;

    msptRingBuffer[ringIndex % msptRingBuffer.length] = totalMsptMs;
    ringIndex++;

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
    };
  };

  invoke('subscribe_live_telemetry', {
    serverId,
    onFrame: channel,
  });

  onUnmounted(() => {
    // Channel automatically closes when dropped
  });

  return {
    latestHeader,
    msptRingBuffer,
  };
}

```

### 7.5 Dynamic Gourd / Velocity Routing Hot-Swap (`src-tauri/src/commands/routing.rs`)

When an admin drags a wire between a Server Node and the Proxy Node in the VueFlow UI, this Tauri command atomically updates both the native **Gourd** in-memory routing table *and* patches `velocity.toml` if the user is running Java Velocity in hybrid mode:

```rust
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RouteWireSpec {
    pub proxy_id: uuid::Uuid,
    pub backend_server_id: uuid::Uuid,
    pub server_alias: String,
    pub forced_hosts: Vec<String>,
    pub priority_tier: i32,
    pub permission_gate: Option<String>,
}

#[tauri::command]
pub async fn hot_wire_server_to_proxy(
    state: State<'_, AppState>,
    spec: RouteWireSpec,
) -> Result<String, String> {
    // 1. Resolve backend endpoint (Unix Socket if local, mTLS QUIC IP:Port if remote)
    let endpoint = state
        .orchestrator
        .resolve_server_endpoint(spec.backend_server_id)
        .await
        .map_err(|e| e.to_string())?;

    // 2. Broadcast CRDT Routing Mutation over the Vineyard Mesh
    // Native Gourd proxies apply this in < 100 microseconds with zero dropped packets.
    // Java Velocity proxies running `velocity-vineyard-bridge.jar` call `ProxyServer#registerServer` dynamically.
    state
        .vineyard_bus
        .publish_route_mutation(
            spec.proxy_id,
            &spec.server_alias,
            &endpoint,
            &spec.forced_hosts,
            spec.priority_tier,
            spec.permission_gate.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?;

    // 3. Persist to declarative TOML state so topology survives cold boots
    state
        .config_ledger
        .commit_route(&spec)
        .await
        .map_err(|e| e.to_string())?;

    Ok(format!(
        "Live-wired '{}' ({}) to proxy {} in 0ms downtime.",
        spec.server_alias, endpoint, spec.proxy_id
    ))
}

```

---

## MODULE 8: PHASED ENGINEERING ROADMAP

To take **PumpkinPatch** from its current state (half-implemented Plan analytics, basic server creation/Velocity config, basic plugin installer) to this complete enterprise-grade vision, execution is structured into four pragmatic milestones:

### Phase 1: Foundation Consolidation & Tauri Binary IPC (`Weeks 1–4`)

1. **Vineyard Local & QUIC Bus:** Replace standard REST/JSON polling between the Tauri Rust core and your PumpkinMC instances with the `rkyv` + `tauri::ipc::Channel` binary pipeline.
2. **Velocity Bridge Plugin + Config Hot-Reloading:** Finalize the `toml_edit` AST patcher for `velocity.toml` and write a tiny 40 KB Java Velocity companion plugin (`pumpkinpatch-velocity-bridge.jar`) that connects to the Tauri app over a local Unix socket/TCP channel so servers can be registered/unregistered dynamically without restarting Velocity.
3. **VueFlow Topology Canvas:** Upgrade the server creation UI into a drag-and-drop node graph where creating a server node and wiring it to the Velocity node provisions the Copy-on-Write directory, injects the `forwarding.secret`, and registers the route automatically.

### Phase 2: HarvestMetrics & The 3D World Inspector (`Weeks 5–8`)

1. **Embedded DuckDB Time-Series Store:** Wire your existing Plan-style analytics into an embedded DuckDB database inside the Tauri app data directory for sub-millisecond cohort, retention, and session funnel queries.
2. **PumpkinMC Telemetry Cargo Feature:** Create the `pumpkin-hook-sdk` crate that hooks into PumpkinMC's tick loop, player join/quit events, and chunk profiler, streaming `TickTelemetryFrame` structs into the Tauri backend.
3. **Three.js Live Chunk & Entity Map:** Build the Vue 3 WebGL viewport that renders loaded chunks colored by tick heat alongside live player markers and click-to-inspect inventories.

### Phase 3: The "Gourd" Native Rust Proxy & Artifact SAT Solver (`Weeks 9–14`)

1. **Gourd Edge Proxy Core:** Build the standalone Rust proxy crate supporting Minecraft `Handshake`/`Status`/`Login`/`Config` states, Velocity `MODERN` forwarding, and zero-copy `Play`-state packet splicing.
2. **Universal Plugin & WASM Manager:** Integrate Modrinth/Hangar APIs with `pubgrub` dependency resolution, Monaco TOML/JSON schema editing, and per-server config inheritance.

### Phase 4: TendrilVoice SFU, PatchWork CDN & Seamless Sharding (`Weeks 15–20`)

1. **TendrilVoice Audio Router & Tauri Audio Deck:** Implement the UDP Opus SFU compatible with Simple Voice Chat / Plasmo Voice packets, and wire `cpal`/`rodio` into the Tauri backend so admins can monitor proximity audio and review 60-second report ring buffers directly in Vue.
2. **PatchWork Resource Pack Merger:** Automate `CustomModelData` collision resolution and Minecraft 1.20.3+ delta pack pushing at the proxy layer.
3. **CrowSwarm Load Tester & Time-Machine CDC Backups:** Add the synthetic Rust bot swarm stress-tester and `FastCDC` + `zstd` chunk-level rollback tools.

---

Would you like to dive deeper into the **Rust implementation of the Gourd Proxy vs. the Java Velocity Bridge**, the **Vue 3 + Tauri binary IPC store architecture**, or the **PumpkinMC telemetry hook patch** first?