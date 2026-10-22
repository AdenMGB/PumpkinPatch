To port **Plan (Player Analytics)** (`com.djrapitops.plan`) to **PumpkinMC**, you are bridging one of the most comprehensive telemetry, time-series, and web-dashboarding engines in the Java Minecraft ecosystem over to a high-performance, asynchronous Rust server environment.

---

## 1. Executive Summary & Architectural Paradigms

**Plan (Player Analytics)** is not merely a statistics logger; it is a full-stack, distributed telemetry and business-intelligence suite embedded directly inside a Minecraft server process. In its native Java implementation, Plan operates across standalone servers (Bukkit, Spigot, Paper, Folia, Sponge, Fabric, NukkitX) and proxy networks (BungeeCord, Velocity) using a shared dependency-injection graph powered by **Dagger 2**.

### 1.1 How Plan Operates Under the Hood

At runtime, Plan splits its workload into five distinct operational planes:

1. **The Telemetry & Event Interception Plane:** Low-overhead listeners attached to the server’s main tick loop and event bus. They capture raw state transitions (joins, quits, world changes, movement vectors, combat, chat, command execution, and tick durations) and immediately offload them from the main server thread.
2. **The In-Memory Session & State Cache:** Rather than executing SQL `UPDATE` queries on every player action, Plan maintains an active `SessionCache` and `AFKTracker` in memory. Active sessions accumulate playtime, AFK slices, per-world/per-gamemode durations, player kills, mob kills, and deaths in volatile memory until the session terminates (or the server shuts down), at which point the entire session object is committed atomically.
3. **The Asynchronous Transaction Queue & Persistence Layer:** All database writes are encapsulated as `Transaction` objects containing `Executable` SQL batches. They are queued into an asynchronous single-threaded executor (backed by HikariCP connection pooling for MySQL/MariaDB, SQLite, or H2) that guarantees chronological write ordering, automatic schema patching, and zero main-thread I/O blocking.
4. **The Analytical & Mathematical Engine:** When data is requested, Plan executes complex relational aggregations and feeds the raw time-series arrays into mathematical processors (`ActivityIndex`, `LowPassFilter`, linear regression trendlines, retention cohort calculators, and cardinal graph downsamplers).
5. **The Embedded HTTP/HTTPS REST Server & React SPA:** Plan embeds an internal Jetty/Undertow/Sun HTTP server (default port `8804`) that serves a compiled **React Single Page Application (SPA)** along with dozens of RESTful `/v1/*` JSON endpoints, complete with PBKDF2/BCrypt user authentication, RBAC permission groups, session cookies, and CORS policies.

### 1.2 Target Architecture on PumpkinMC

When porting Plan to **PumpkinMC** (written in Rust), you have two architectural pathways:

```
+-----------------------------------------------------------------------------------+
|                            PUMPKINMC SERVER PROCESS (RUST)                        |
+-----------------------------------------------------------------------------------+
|                                                                                   |
|  +-----------------------------+               +-------------------------------+  |
|  |   PATH A: NATIVE RUST PORT  |               |   PATH B: PATCHBUKKIT BRIDGE  |  |
|  |   (Pure Rust Crate / WIT)   |               |   (Embedded JVM + Protobuf)   |  |
|  +--------------+--------------+               +---------------+---------------+  |
|                 |                                              |                  |
|                 v                                              v                  |
|  * Non-blocking EventHandlers                  * PatchBukkitEventHandler (Rust)   |
|  * Tokio Async MPSC Channels                   * Protobuf FFI Serialization       |
|  * DashMap Active Session Cache                * JvmWorker -> Bukkit Event Bus    |
|  * SQLx (Postgres/MySQL/SQLite)                * Unmodified Plan.jar Execution    |
|  * Axum / Actix-Web REST + SPA                 * Native FFM Callbacks for TPS     |
|                                                                                   |
+-----------------------------------------------------------------------------------+

```

* **Path A: Native Rust Port (`pumpkin-plan` Plugin):** Built directly against PumpkinMC's Rust plugin system (`pumpkin_api_macros`, `Plugin` trait, `Context`, or WebAssembly WIT bindings). You replace Dagger 2 with Rust struct composition (`Arc<AppState>`), Java's `ConcurrentHashMap` with `DashMap`, HikariCP/JDBC with `sqlx` (supporting SQLite, MySQL/MariaDB, and PostgreSQL), scheduled executors with `tokio::spawn` and `tokio::time::interval`, and the embedded Java webserver with **Axum** or **Actix-Web**.
* **Path B: PatchBukkit JVM Bridge Adaptation:** PumpkinMC features **PatchBukkit**, which embeds a JVM worker and communicates across a Rust-to-Java boundary via JNI and the Foreign Function & Memory (FFM) API using Protocol Buffers (`.proto`). To run the original Java `Plan.jar` via PatchBukkit, you must ensure all Bukkit API surfaces Plan relies upon (specifically Paper's `Server.getTPS()`, `Server.getTickTimes()`, `Player.getPing()`, `AsyncPlayerChatEvent`, and `PlayerLoginEvent.getHostname()`) are mapped in PatchBukkit's `.proto` definitions and `NativeBridgeFfi` callbacks.

This specification provides the complete domain model, formulas, schemas, endpoints, and architectural mappings required to build a 1:1 native Rust port (Path A) or complete the PatchBukkit bindings (Path B).

---

## 2. Exhaustive Telemetry & Metrics Catalog

Every piece of data Plan collects belongs to one of eight domains. Below is the exact specification of every metric, its trigger mechanism, in-memory representation, and persistence rules.

---

### 2.1 Player Identity & Profile Telemetry

| Metric / Field | Data Type | Trigger / Source | Persistence / Storage |
| --- | --- | --- | --- |
| **Player UUID** | `UUID` (36-char string / 128-bit) | `PlayerLoginEvent` / Handshake packet | `plan_users.uuid` (Unique Index) |
| **Current Username** | `VARCHAR(36)` | `PlayerLoginEvent` / `PlayerJoinEvent` | `plan_users.name` (Updated on join if changed) |
| **First Registration Date (Global)** | `BIGINT` (Epoch ms) | First time UUID is seen on *any* network node | `plan_users.registered` |
| **First Registration Date (Per-Server)** | `BIGINT` (Epoch ms) | First time UUID joins a specific `server_id` | `plan_user_info.registered` |
| **Operator Status (`OP`)** | `BOOLEAN` | Checked on join, leave, and `/op` `/deop` | `plan_user_info.opped` |
| **Banned Status** | `BOOLEAN` | Checked on join, kick, leave, and ban events | `plan_user_info.banned` |
| **Join Address (Virtual Host)** | `VARCHAR(191)` | Handshake packet `server_address` (e.g., `mc.example.com`) | `plan_join_address.join_address` (Normalized) |
| **Display Nickname** | `VARCHAR(255)` | Chat events or display name change per server | `plan_nicknames.nickname` + `last_used` (Epoch ms) |
| **Kick Count** | `INTEGER` | `PlayerKickEvent` | `plan_users.times_kicked` (Incremented by `+1`) |

#### Implementation Mechanics: Join Address Normalization

When a player connects, the Minecraft handshake packet contains the hostname they typed into their multiplayer client (plus optional Forge/FML markers or BungeeCord IP forwarding strings separated by null bytes `\0`). Plan cleans this string before storing it:

1. Split the raw hostname string at `\0` (null byte) and take index `0` to strip BungeeCord/Velocity forwarding payloads and FML markers (`\0FML\0`, `\0FML2\0`).
2. Strip any trailing port delimiter (`:25565`) and trailing dots (`.`).
3. If `Data_gathering.Join_addresses.Preserve_case` is `false` (default), convert the entire string to lowercase.
4. Validate that the string is a valid IPv4, IPv6, or RFC-compliant DNS hostname (unless `Preserve_invalid` is `true`).
5. Check against `Data_gathering.Join_addresses.Filter_out_from_data` list; if matched, replace with `"Unknown"`.
6. Map the string into the `plan_join_address` lookup table and associate the `join_address_id` with both `plan_user_info` (latest join address) and the active `plan_sessions` record.

---

### 2.2 Session Lifecycle, AFK Engine & World/Gamemode Matrices

A **Session** is the foundational atomic unit of player analytics in Plan. It represents a single continuous connection to a single game server.

#### The In-Memory Active Session Object

When a player joins (`PlayerJoinEvent`), Plan instantiates an `ActiveSession` struct in memory:

```rust
pub struct ActiveSession {
    pub user_uuid: Uuid,
    pub server_uuid: Uuid,
    pub session_start: i64,          // Epoch milliseconds
    pub session_end: Option<i64>,    // Populated on quit/shutdown
    pub afk_time: i64,               // Accumulated milliseconds spent AFK
    pub mob_kills: u32,              // Count of non-player living entities killed
    pub deaths: u32,                 // Count of player deaths during this session
    pub join_address: String,        // Normalized virtual host used for this login
    pub world_times: WorldTimes,     // Nested state machine for World x Gamemode time
    pub player_kills: Vec<PlayerKill>, // Detailed list of PvP kills in this session
}

```

#### The AFK Tracking State Machine (`AFKTracker`)

Plan does not rely on third-party plugins (like EssentialsX) to calculate AFK time; it runs its own internal movement and action tracker so AFK metrics remain consistent across all platforms.

* **AFK Threshold (`T_afk`):** Configurable via `Time.Thresholds.AFK_threshold` (Default: `3 MINUTES` = `180,000 ms`).
* **Tracked Activity Events:**
1. **Camera Rotation / Movement:** When a player moves or rotates their head (`PlayerMoveEvent`). To prevent micro-jitter or water-stream AFK pools from resetting AFK, Plan specifically checks if the **pitch** or **yaw** changed, or if block coordinates changed intentionally.
2. **Chat Execution:** Sending a chat message.
3. **Command Execution:** Running any `/` command.
4. **Sneaking / Sprinting / Interaction:** Toggling sneak or interacting with blocks/entities.


* **Algorithm:**
* Maintain `last_movement: HashMap<Uuid, i64>` storing the timestamp (`t_last`) of the player's last action.
* When a new action occurs at timestamp `t_now`:

$$\Delta t = t_{\text{now}} - t_{\text{last}}$$


$$\text{If } \Delta t > T_{\text{afk}}, \text{ then } \text{session.afk\_time} \mathrel{+}= \Delta t$$


* Update $t_{\text{last}} = t_{\text{now}}$.
* **Permission Bypass:** If the player holds the permission node `plan.ignore.afk`, $t_{\text{last}}$ is marked with a special sentinel value (`-1`), and $\Delta t$ is never added to `afk_time`.
* **Session Termination:** When the player disconnects at $t_{\text{quit}}$, perform one final check: if $t_{\text{quit}} - t_{\text{last}} > T_{\text{afk}}$, add $(t_{\text{quit}} - t_{\text{last}})$ to `session.afk_time`.


* **Derived Playtime Formulas:**

$$\text{Total Session Length} = t_{\text{end}} - t_{\text{start}}$$


$$\text{Active Playtime} = (t_{\text{end}} - t_{\text{start}}) - \text{afk\_time}$$



#### World & Gamemode Time Multiplexing (`WorldTimes`)

Plan tracks exactly how many milliseconds a player spends in **each Gamemode inside each World** during every single session.

* **Tracked Gamemodes:** `SURVIVAL`, `CREATIVE`, `ADVENTURE`, `SPECTATOR`.
* **Data Structure:** A map of `(WorldName, GameMode) -> u64 (milliseconds)`, alongside `current_world: String`, `current_gamemode: GameMode`, and `last_state_change: i64`.
* **State Transition Triggers:**
1. `PlayerJoinEvent`: Initialize `current_world`, `current_gamemode`, and `last_state_change = t_join`.
2. `PlayerChangedWorldEvent`: Calculate $\Delta t = t_{\text{now}} - \text{last\_state\_change}$. Add $\Delta t$ to `(old_world, current_gamemode)`. Set `current_world = new_world` and `last_state_change = t_now`.
3. `PlayerGameModeChangeEvent`: Calculate $\Delta t = t_{\text{now}} - \text{last\_state\_change}$. Add $\Delta t$ to `(current_world, old_gamemode)`. Set `current_gamemode = new_gamemode` and `last_state_change = t_now`.
4. `PlayerQuitEvent` / Server Shutdown: Calculate $\Delta t = t_{\text{quit}} - \text{last\_state\_change}$. Add $\Delta t$ to `(current_world, current_gamemode)`.


* **World Aliasing:** Before rendering to the frontend, world names are passed through `World_aliases.List` (direct key-value mapping) and `World_aliases.Regex` (e.g., `"Minigame Worlds:^arena_[0-9]+$"`). Multiple physical worlds mapped to the same alias have their `SURVIVAL`, `CREATIVE`, `ADVENTURE`, and `SPECTATOR` milliseconds summed together into a single logical world slice.

---

### 2.3 Combat, PvP, PvE & Weapon Normalization

Plan separates player-vs-player (PvP) kills from player-vs-environment (PvE) mob kills and deaths.

#### PvP Kills (`plan_kills`)

When a `Player` dies and the killer is resolved to another `Player` (either via direct melee attack, or by tracing the `ProjectileSource` of an arrow/trident/firework, or via tamed wolf/entity ownership):

* **Killer UUID:** UUID of the attacking player.
* **Victim UUID:** UUID of the deceased player.
* **Server UUID:** UUID of the server where the kill occurred.
* **Session ID:** Foreign key linking the kill to the killer's active session (`plan_sessions.id`).
* **Timestamp (`date`):** Epoch milliseconds of the kill.
* **Weapon String (`weapon`):** Extracted from the item in the killer's main hand (or the projectile/entity type).
* **Weapon Name Normalization:** Plan strips Minecraft formatting codes (`§a`, hex colors), extracts the material registry name (e.g., `DIAMOND_SWORD` $\rightarrow$ `"Diamond Sword"`), or uses the entity name (`"Arrow"`, `"Trident"`, `"End Crystal"`, `"Bed"`).



#### PvE & Deaths (Session Aggregates)

To prevent database bloat on mob-grinder servers, individual mob kills and environmental deaths are **not** stored as rows in `plan_kills`. Instead:

* When a `LivingEntity` (that is not a `Player` or `ArmorStand`) dies and its killer is a `Player`, `active_session.mob_kills` is incremented by `1`.
* Whenever a `Player` dies from any cause (PvP, mob, fall damage, lava, `/kill`), that victim's `active_session.deaths` is incremented by `1`.
* **Kill/Death Ratio (KDR) Formulas:**

$$\text{PvP KDR} = \frac{\text{Total Player Kills}}{\max(1, \text{Total Player-Caused Deaths})}$$


$$\text{Overall Player KDR} = \frac{\text{Total Player Kills}}{\max(1, \text{Total Deaths (All Causes)})}$$


$$\text{Mob KDR} = \frac{\text{Total Mob Kills}}{\max(1, \text{Total Mob-Caused / Total Deaths})}$$



---

### 2.4 Network, Ping & Geolocation Telemetry

#### Geolocation Resolution (`plan_geolocations`)

When a player connects (`PlayerLoginEvent`), Plan extracts their `InetAddress` (IPv4 or IPv6):

1. **Local/Private IP Check:** If the IP is loopback (`127.0.0.1`, `::1`) or RFC 1918 private (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`), it resolves immediately to `"Local Machine"`.
2. **Primary Resolver (MaxMind GeoLite2 `.mmdb`):** If `Data_gathering.Accept_GeoLite2_EULA` is `true`, Plan downloads and caches `GeoLite2-Country.mmdb` from `[https://geodb.playeranalytics.net/GeoLite2-Country.mmdb](https://geodb.playeranalytics.net/GeoLite2-Country.mmdb)`. It performs a binary tree lookup on the IP address to extract the English Country Name (e.g., `"United States"`, `"Germany"`, `"Australia"`).
3. **Fallback Resolver (`ip-api.com`):** If GeoLite2 is disabled or unavailable, Plan performs an HTTP GET request to `[http://ip-api.com/json/](http://ip-api.com/json/){ip}?fields=status,country,countryCode` on a background thread.
4. **Privacy & GDPR Anonymization:** Plan **never** stores the raw IP address in the database. For security/troubleshooting logs or unique IP hashing, it zeroes out the last octet of IPv4 (`192.0.2.xxx` $\rightarrow$ `192.0.2.0`) and the last 80 bits of IPv6 (`2001:db8::`). In `plan_geolocations`, only `(user_id, geolocation, last_used)` is stored.

#### Ping / Latency Sampling (`plan_ping`)

Ping can fluctuate wildly during login chunk loading. Plan enforces two delays before sampling a player's latency:

* **Server Enable Delay:** `Time.Delays.Ping_server_enable_delay` (Default: `300 SECONDS` after server boot).
* **Player Join Delay:** `Time.Delays.Ping_player_join_delay` (Default: `30 SECONDS` after `PlayerJoinEvent`).
* **Sampling Cadence:** Once a player has been online for $>30\text{ seconds}$, a repeating task samples their current latency (`player.getPing()` in ms) every **2 seconds** (30 samples per minute).
* **Aggregation Window:** After accumulating **30 samples** (60 seconds of data), Plan computes three values for that 1-minute window:
1. `max_ping`: Maximum sample in the 60s window.
2. `min_ping`: Minimum sample in the 60s window.
3. `avg_ping`: Arithmetic mean of the 30 samples.


* These three integers are written as a single row into `plan_ping` with `(user_id, server_id, date, max_ping, min_ping, avg_ping)`.

---

### 2.5 Server Hardware, JVM/Process & World Performance (`plan_tps`)

Despite being named `plan_tps`, this table is the master **60-second time-series performance log** for the entire server.

#### Sampling Cadence & Aggregation

A background task runs every **1 second** (20 ticks) to sample fast-changing gauges, and every **60 seconds** it aggregates the 60 samples and flushes one row to `plan_tps`:

| Column / Metric | Data Type | Aggregation Method (over 60s) | Source / Calculation |
| --- | --- | --- | --- |
| `server_id` | `INTEGER` | Constant | Foreign key to `plan_servers.id` |
| `date` | `BIGINT` | Timestamp at flush | Epoch milliseconds |
| `ticks_per_second` | `DOUBLE` | **Average** of 60 samples | Calculated from tick delta time (capped at `20.0`) |
| `players_online` | `INTEGER` | **Maximum** of 60 samples | Peak concurrent players during that minute |
| `cpu_usage` | `DOUBLE` | **Average** of 60 samples | System/Process CPU load percentage (`0.0` to `100.0`) |
| `ram_usage` | `BIGINT` | **Average** of 60 samples | Used memory in Megabytes (`MB`) |
| `entities` | `INTEGER` | **Average** of 60 samples | Sum of loaded entities across all worlds |
| `chunks_loaded` | `INTEGER` | **Average** of 60 samples | Sum of loaded chunks across all worlds |
| `free_disk_space` | `BIGINT` | **Minimum** of 60 samples | Usable free disk space on the server partition in Megabytes (`MB`) |
| `mspt_average` | `DOUBLE` | **Average** over 60s | Mean Milliseconds Per Tick (`MSPT`) during the minute |
| `mspt_95th_percentile` | `DOUBLE` | **95th Percentile** over 60s | 95th percentile tick duration in milliseconds (`P95 MSPT`) |

#### MSPT & 95th Percentile Calculation

In 1.20+ / Paper / PumpkinMC, raw TPS (`20.0`) masks micro-stutters if the server catches up on subsequent ticks. Plan records the duration in nanoseconds/milliseconds of every individual tick (up to $20 \times 60 = 1,200$ ticks per minute):

* **`mspt_average`:**

$$\text{MSPT}_{\text{avg}} = \frac{1}{N} \sum_{i=1}^{N} \text{tick\_duration}_i$$


* **`mspt_95th_percentile`:** Sort the array of $N$ tick durations in ascending order: $T_{(1)} \le T_{(2)} \le \dots \le T_{(N)}$. Select the element at index $\lceil 0.95 \times N \rceil$. If a single tick spikes to `150ms` due to chunk generation or garbage collection, `mspt_95th_percentile` immediately surfaces the lag spike even if average TPS remains `20.0`.

#### Server Uptime & Downtime Detection

Plan determines **Server Uptime** dynamically:

* Current Uptime = `now - server_enable_timestamp`.
* Historical Downtime / Restart Detection: When rendering the Performance graph, if the time delta between two consecutive `plan_tps` rows exceeds `3 minutes` ($\Delta t > 180,000\text{ ms}$) and `Display_options.Graphs.Show_gaps_in_data` is `true`, Plan injects `null` points into the time-series array to visually break the line chart and denote server downtime.

---

### 2.6 Command Usage Tracking (`plan_commandusages`)

Whenever a player executes a command (`PlayerCommandPreprocessEvent`):

1. Check if the player has permission `plan.ignore.commanduse`. If `true`, abort.
2. Extract the root command label (index `0` of space-separated arguments, stripping the leading `/`, converted to lowercase, max 20 characters).
3. Check if the command is registered on the server:
* If registered and `Data_gathering.Commands.Log_aliases_as_main_command` is `true`, resolve the alias to its canonical command name (e.g., `/msg` or `/w` $\rightarrow$ `tell`).
* If unregistered and `Data_gathering.Commands.Log_unknown` is `false`, discard it (prevents typos containing passwords like `/l mysecretpassword` from ever entering the database).


4. Increment `times_used` by `+1` in `plan_commandusages` for `(server_id, command)`.

---

## 3. Advanced Analytical Models & Mathematical Formulas

To achieve parity with Plan, your port must implement its exact analytical models. Plan's frontend expects pre-computed indices and regressions from the backend API.

---

### 3.1 The Player Activity Index (PAI)

The **Activity Index** is Plan's signature mathematical model. Instead of using a naive "logged in within the last $X$ days" boolean, Plan computes a continuous score from **`0.0` to `5.0**` based on the player's playtime across the **last 3 weeks**, weighted linearly so recent weeks matter more than older weeks, and normalized via a asymptotic bell/logistic curve.

#### Step 1: Time Window Partitioning

Given a calculation timestamp $t_{\text{now}}$ and `ONE_WEEK` = $604,800,000\text{ ms}$ (7 days):

* **Week 1 (Most Recent):** $[t_{\text{now}} - 1\text{ week},\ t_{\text{now}}]$ $\longrightarrow$ Active Playtime = $P_1$
* **Week 2 (Middle):** $[t_{\text{now}} - 2\text{ weeks},\ t_{\text{now}} - 1\text{ week}]$ $\longrightarrow$ Active Playtime = $P_2$
* **Week 3 (Oldest):** $[t_{\text{now}} - 3\text{ weeks},\ t_{\text{now}} - 2\text{ weeks}]$ $\longrightarrow$ Active Playtime = $P_3$

*(Note: Playtime $P_i$ is **Active Playtime** in milliseconds, meaning `session_length - afk_time` inside each window).*

#### Step 2: Threshold Normalization & Asymptotic Curve

Let $T$ be the configured weekly playtime threshold (`Time.Thresholds.Activity_index.Playtime_threshold`, default: `30 MINUTES` = $1,800,000\text{ ms}$).

For each week $i \in \{1, 2, 3\}$, compute the intermediate non-linear activity ratio $y_i$:


$$y_i = 1.0 - \frac{1.0}{\frac{\pi}{2} \cdot \left(\frac{P_i}{T}\right) + 1.0}$$

Why this formula?

* When $P_i = 0$, $y_i = 0.0$.
* When $P_i = T$ (the player hit the exact threshold), $y_i = 1 - \frac{1}{\frac{\pi}{2} + 1} \approx 0.611$.
* As $P_i \to \infty$, $y_i \to 1.0$ asymptotically, preventing 100-hour grinders from skewing the index to infinity.

#### Step 3: Weighted Weekly Summation & Scaling to `[0.0, 5.0]`

Weight the three weeks using weights $W_1 = 3$, $W_2 = 2$, $W_3 = 1$ (total weight $= 6$), and multiply by $5.0$:

$$\text{Activity Index} = 5.0 \times \frac{3 y_1 + 2 y_2 + 1 y_3}{6}$$

#### Step 4: Activity Group Classification Thresholds

Every player is classified into one of five discrete buckets based on their `Activity Index` score:

| Activity Group | Index Range (`[min, max)`) | Color Hex (Default Theme) | Meaning |
| --- | --- | --- | --- |
| **Very Active** | `>= 3.75` to `5.00` | `#2eab6f` (Green) | Plays consistently above threshold every week |
| **Active** | `>= 3.00` to `< 3.75` | `#82b346` (Light Green) | Plays regularly around or above the weekly threshold |
| **Regular** | `>= 2.00` to `< 3.00` | `#b5b336` (Yellow-Green) | Plays moderately or missed one recent week |
| **Irregular** | `>= 1.00` to `< 2.00` | `#d98b2b` (Orange) | Sporadic login activity or tapering off |
| **Inactive** | `0.00` to `< 1.00` | `#777777` (Gray) | Almost no playtime in the last 21 days |

---

### 3.2 Playerbase Development & Trend Analysis

On the Server and Network overview pages, Plan compares playerbase states over time:

* **30-Day / 7-Day Trend Arrows:** For metrics like *Unique Players*, *New Players*, *Regular Players*, *Average Playtime*, *Average Session Length*, and *Average Peak Players*, Plan computes two windows:
* **Current Window:** $[t_{\text{now}} - \Delta W,\ t_{\text{now}}]$
* **Previous Window:** $[t_{\text{now}} - 2\Delta W,\ t_{\text{now}} - \Delta W]$
* **Trend Delta:** $\Delta = \text{Value}_{\text{current}} - \text{Value}_{\text{previous}}$, rendered with a green upward badge (`+`), red downward badge (`-`), or neutral badge (`=`).


* **New Player Retention Percentage:**

$$\text{Retention Rate} = \frac{\text{Count of players registered in window } W \text{ with } \ge 2 \text{ sessions (or playtime } > T\text{)}}{\text{Total players registered in window } W} \times 100\%$$


* **Join Address Retention & Stickiness:** Groups players by their initial `join_address` and correlates it with their current `Activity Index` and cumulative playtime to show server owners which advertising domains (e.g., `topg.example.com` vs `tiktok.example.com`) yield players who actually stay.

---

### 3.3 Time-Series Downsampling & Low-Pass Filtering

When rendering 30 days or 365 days of 1-minute `plan_tps` data, sending $525,600$ JSON objects to the browser would crash the React frontend. Plan applies signal-processing algorithms on the backend before serializing graphs:

1. **Douglas-Peucker / Bucket Downsampling:** Reduces high-density time-series points from regions older than 7 days into larger time buckets while preserving local minima (TPS drops) and local maxima (player count peaks).
2. **Timezone Offset Adjustment:** All timestamps in the database are stored in **UTC Epoch Milliseconds**. Before generating daily/hourly histograms (such as the **PunchCard** graph), Plan applies the configured `Formatting.Dates.TimeZone` offset so "12:00 PM" aligns with the server's local noon.
3. **The PunchCard Matrix ($7 \times 24$ Grid):**
* Aggregates all historical sessions over the last 30 days into a 2D matrix of `DayOfWeek (0..6, Monday..Sunday) x HourOfDay (0..23)`.
* The radius of each bubble at $(d, h)$ is scaled proportionally to the number of unique active sessions overlapping that specific hour of that day of the week.



---

## 4. Complete Relational Database Schema Specification

Plan supports **SQLite**, **MySQL / MariaDB**, and **H2** out of the box (and in a Rust port via `sqlx`, you can easily support **SQLite**, **MySQL/MariaDB**, and **PostgreSQL**).

Below is the complete DDL specification of all core, security, and extension tables in Plan's schema.

---

### 4.1 Core Telemetry Tables

#### 1. `plan_servers`

Stores every standalone game server and proxy node in the network.

```sql
CREATE TABLE plan_servers (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    uuid VARCHAR(36) NOT NULL UNIQUE,
    name VARCHAR(100),
    web_address VARCHAR(150),
    is_installed BOOLEAN NOT NULL DEFAULT 1,
    is_proxy BOOLEAN NOT NULL DEFAULT 0,
    max_players INTEGER NOT NULL DEFAULT -1,
    plan_version VARCHAR(18) NOT NULL DEFAULT 'Unknown'
);

```

#### 2. `plan_users`

Global registry of unique players across the entire network.

```sql
CREATE TABLE plan_users (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    uuid VARCHAR(36) NOT NULL UNIQUE,
    registered BIGINT NOT NULL,
    name VARCHAR(36) NOT NULL,
    times_kicked INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX plan_users_uuid_index ON plan_users(uuid);

```

#### 3. `plan_user_info`

Per-server metadata for each player (links a `user_id` to a specific `server_id`).

```sql
CREATE TABLE plan_user_info (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    user_id INTEGER NOT NULL,
    server_id INTEGER NOT NULL,
    join_address VARCHAR(191),
    registered BIGINT NOT NULL,
    opped BOOLEAN NOT NULL DEFAULT 0,
    banned BOOLEAN NOT NULL DEFAULT 0,
    FOREIGN KEY (user_id) REFERENCES plan_users(id) ON DELETE CASCADE,
    FOREIGN KEY (server_id) REFERENCES plan_servers(id) ON DELETE CASCADE,
    UNIQUE (user_id, server_id)
);

```

#### 4. `plan_join_address`

Lookup table of normalized virtual hostnames used to join the server.

```sql
CREATE TABLE plan_join_address (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    join_address VARCHAR(191) UNIQUE
);

```

#### 5. `plan_sessions`

Stores every completed session. `join_address_id` references `plan_join_address(id)`.

```sql
CREATE TABLE plan_sessions (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    user_id INTEGER NOT NULL,
    server_id INTEGER NOT NULL,
    session_start BIGINT NOT NULL,
    session_end BIGINT NOT NULL,
    mob_kills INTEGER NOT NULL DEFAULT 0,
    deaths INTEGER NOT NULL DEFAULT 0,
    afk_time BIGINT NOT NULL DEFAULT 0,
    join_address_id INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (user_id) REFERENCES plan_users(id) ON DELETE CASCADE,
    FOREIGN KEY (server_id) REFERENCES plan_servers(id) ON DELETE CASCADE,
    FOREIGN KEY (join_address_id) REFERENCES plan_join_address(id)
);
CREATE INDEX plan_sessions_date_index ON plan_sessions(session_start, session_end);
CREATE INDEX plan_sessions_user_id_index ON plan_sessions(user_id);
CREATE INDEX plan_sessions_server_id_index ON plan_sessions(server_id);

```

#### 6. `plan_worlds`

Lookup table of world names per server.

```sql
CREATE TABLE plan_worlds (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    world_name VARCHAR(100) NOT NULL,
    server_id INTEGER NOT NULL,
    FOREIGN KEY (server_id) REFERENCES plan_servers(id) ON DELETE CASCADE
);

```

#### 7. `plan_world_times`

Stores the millisecond breakdown per gamemode per world for every individual session.

```sql
CREATE TABLE plan_world_times (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    user_id INTEGER NOT NULL,
    world_id INTEGER NOT NULL,
    server_id INTEGER NOT NULL,
    session_id INTEGER NOT NULL,
    survival_time BIGINT NOT NULL DEFAULT 0,
    creative_time BIGINT NOT NULL DEFAULT 0,
    adventure_time BIGINT NOT NULL DEFAULT 0,
    spectator_time BIGINT NOT NULL DEFAULT 0,
    FOREIGN KEY (user_id) REFERENCES plan_users(id) ON DELETE CASCADE,
    FOREIGN KEY (world_id) REFERENCES plan_worlds(id) ON DELETE CASCADE,
    FOREIGN KEY (server_id) REFERENCES plan_servers(id) ON DELETE CASCADE,
    FOREIGN KEY (session_id) REFERENCES plan_sessions(id) ON DELETE CASCADE
);
CREATE INDEX plan_world_times_session_id_index ON plan_world_times(session_id);
CREATE INDEX plan_world_times_user_id_index ON plan_world_times(user_id);

```

#### 8. `plan_kills`

Stores individual PvP kill events.

```sql
CREATE TABLE plan_kills (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    killer_uuid VARCHAR(36) NOT NULL,
    victim_uuid VARCHAR(36) NOT NULL,
    server_uuid VARCHAR(36) NOT NULL,
    session_id INTEGER NOT NULL,
    date BIGINT NOT NULL,
    weapon VARCHAR(45) NOT NULL,
    FOREIGN KEY (session_id) REFERENCES plan_sessions(id) ON DELETE CASCADE
);
CREATE INDEX plan_kills_killer_index ON plan_kills(killer_uuid);
CREATE INDEX plan_kills_victim_index ON plan_kills(victim_uuid);
CREATE INDEX plan_kills_date_index ON plan_kills(date);

```

#### 9. `plan_ping`

Stores 60-second aggregated latency windows per player per server.

```sql
CREATE TABLE plan_ping (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    user_id INTEGER NOT NULL,
    server_id INTEGER NOT NULL,
    date BIGINT NOT NULL,
    max_ping INTEGER NOT NULL,
    min_ping INTEGER NOT NULL,
    avg_ping DOUBLE NOT NULL,
    FOREIGN KEY (user_id) REFERENCES plan_users(id) ON DELETE CASCADE,
    FOREIGN KEY (server_id) REFERENCES plan_servers(id) ON DELETE CASCADE
);
CREATE INDEX plan_ping_date_index ON plan_ping(date);
CREATE INDEX plan_ping_user_id_index ON plan_ping(user_id);

```

#### 10. `plan_tps`

Stores 60-second server performance time-series snapshots.

```sql
CREATE TABLE plan_tps (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    server_id INTEGER NOT NULL,
    date BIGINT NOT NULL,
    ticks_per_second DOUBLE NOT NULL,
    players_online INTEGER NOT NULL,
    cpu_usage DOUBLE NOT NULL,
    ram_usage BIGINT NOT NULL,
    entities INTEGER NOT NULL,
    chunks_loaded INTEGER NOT NULL,
    free_disk_space BIGINT NOT NULL,
    mspt_average DOUBLE NOT NULL DEFAULT -1,
    mspt_95th_percentile DOUBLE NOT NULL DEFAULT -1,
    FOREIGN KEY (server_id) REFERENCES plan_servers(id) ON DELETE CASCADE
);
CREATE INDEX plan_tps_date_index ON plan_tps(server_id, date);

```

#### 11. `plan_geolocations`

Stores country-level geolocation history per user.

```sql
CREATE TABLE plan_geolocations (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    user_id INTEGER NOT NULL,
    geolocation VARCHAR(50) NOT NULL,
    last_used BIGINT NOT NULL DEFAULT 0,
    FOREIGN KEY (user_id) REFERENCES plan_users(id) ON DELETE CASCADE,
    UNIQUE (user_id, geolocation)
);

```

#### 12. `plan_nicknames`

Stores in-game display names / nicknames used by players per server.

```sql
CREATE TABLE plan_nicknames (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    uuid VARCHAR(36) NOT NULL,
    server_uuid VARCHAR(36) NOT NULL,
    nickname VARCHAR(255) NOT NULL,
    last_used BIGINT NOT NULL
);

```

#### 13. `plan_commandusages`

Stores command execution counters per server.

```sql
CREATE TABLE plan_commandusages (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    server_id INTEGER NOT NULL,
    command VARCHAR(20) NOT NULL,
    times_used INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (server_id) REFERENCES plan_servers(id) ON DELETE CASCADE,
    UNIQUE (server_id, command)
);

```

#### 14. `plan_settings`

Used in multi-server networks to synchronize configuration YAML payloads from the proxy (`Bungee`/`Velocity`) to backend game servers via the database.

```sql
CREATE TABLE plan_settings (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    server_uuid VARCHAR(36) NOT NULL UNIQUE,
    updated BIGINT NOT NULL,
    content TEXT NOT NULL
);

```

---

### 4.2 Web Security, RBAC & Access Audit Tables

#### 15. `plan_security`

Stores registered web panel accounts, password hashes (BCrypt/PBKDF2), linked player UUIDs, and assigned RBAC permission group IDs.

```sql
CREATE TABLE plan_security (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    username VARCHAR(100) NOT NULL UNIQUE,
    salted_pass_hash VARCHAR(100) NOT NULL UNIQUE,
    linked_to_uuid VARCHAR(36) DEFAULT '',
    group_id INTEGER NOT NULL,
    FOREIGN KEY (group_id) REFERENCES plan_web_group(id)
);

```

#### 16. `plan_web_group`

Defines named web permission groups (e.g., `admin`, `staff`, `player`, `guest`).

```sql
CREATE TABLE plan_web_group (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    group_name VARCHAR(100) NOT NULL UNIQUE
);

```

#### 17. `plan_web_permission`

Catalog of granular web panel permission strings (e.g., `page.network`, `page.server`, `page.player.self`, `page.player.other`, `access.query`, `manage.groups`).

```sql
CREATE TABLE plan_web_permission (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    permission VARCHAR(100) NOT NULL UNIQUE
);

```

#### 18. `plan_web_group_to_permission`

Many-to-many junction table linking web groups to web permissions.

```sql
CREATE TABLE plan_web_group_to_permission (
    group_id INTEGER NOT NULL,
    permission_id INTEGER NOT NULL,
    FOREIGN KEY (group_id) REFERENCES plan_web_group(id) ON DELETE CASCADE,
    FOREIGN KEY (permission_id) REFERENCES plan_web_permission(id) ON DELETE CASCADE,
    UNIQUE (group_id, permission_id)
);

```

#### 19. `plan_cookies`

Persists active web login cookies (`S_ID`) so authenticated sessions survive server restarts.

```sql
CREATE TABLE plan_cookies (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    web_username VARCHAR(100) NOT NULL,
    expires BIGINT NOT NULL,
    cookie VARCHAR(64) NOT NULL UNIQUE,
    FOREIGN KEY (web_username) REFERENCES plan_security(username) ON DELETE CASCADE
);

```

#### 20. `plan_access_log`

HTTP access log recording every web request made to the embedded server (automatically pruned after `Security.Access_log.Remove_logs_after_days`).

```sql
CREATE TABLE plan_access_log (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    time BIGINT NOT NULL,
    from_ip VARCHAR(45),
    request_method VARCHAR(8) NOT NULL,
    request_uri TEXT NOT NULL,
    response_code INTEGER NOT NULL
);

```

---

### 4.3 DataExtension (Third-Party Plugin) Tables

Plan uses an Entity-Attribute-Value (EAV) relational subsystem of **11 tables** to persist arbitrary data provided by third-party plugins without requiring schema migrations per plugin:

```
+---------------------------+       +----------------------------+
|   plan_extension_icon     |<------+   plan_extension_plugin    |
+---------------------------+       +-------------+--------------+
                                                  |
                  +-------------------------------+-------------------------------+
                  |                               |                               |
                  v                               v                               v
+-----------------+---------+   +-----------------+----------+   +----------------+-----------+
|   plan_extension_tabs     |   |  plan_extension_providers  |   |   plan_extension_tables    |
+---------------------------+   +-----------------+----------+   +----------------+-----------+
                                                  |                               |
                                  +---------------+---------------+               +-------+
                                  |                               |                       |
                                  v                               v                       v
                +-----------------+----------+   +----------------+-----------+   +-------+--------------------+
                | plan_extension_user_values |   |plan_extension_server_values|   |plan_extension_user_table_..|
                +----------------------------+   +----------------------------+   |plan_extension_server_tabl..|
                                                                                  +----------------------------+

```

1. **`plan_extension_icon`**: `(id, icon_name, family, color)` — Stores FontAwesome icon metadata (`SOLID`, `REGULAR`, `BRANDS`, and semantic color theme).
2. **`plan_extension_plugin`**: `(id, name, last_updated, server_uuid, icon_id)` — Registers each third-party plugin per server.
3. **`plan_extension_tabs`**: `(id, name, element_order, plugin_id, icon_id)` — Defines UI tabs inside a plugin's card when a plugin organizes its metrics into multiple tabs.
4. **`plan_extension_providers`**: `(id, name, text, description, priority, show_in_players_table, groupable, condition_name, provided_condition, format_type, hidden, player_name, plugin_id, icon_id, tab_id)` — Defines the schema of a single metric provider (e.g., `balance`, `town_name`, `mcmmo_power_level`).
5. **`plan_extension_user_values`**: `(id, boolean_value, double_value, percentage_value, long_value, string_value, group_value, user_uuid, provider_id)` — Stores scalar/group values per player.
6. **`plan_extension_server_values`**: `(id, boolean_value, double_value, percentage_value, long_value, string_value, provider_id)` — Stores scalar values at the server level.
7. **`plan_extension_groups`**: `(id, provider_id, user_uuid, group_name)` — Stores multi-group memberships (e.g., LuckPerms groups or Jobs Reborn active jobs) per player.
8. **`plan_extension_tables`**: `(id, name, col_1_name..col_5_name, col_1_format..col_5_format, plugin_id, icon_id_1..icon_id_5, tab_id, table_color)` — Defines custom 1-to-5 column tables exposed by plugins.
9. **`plan_extension_user_table_values`**: `(id, user_uuid, table_id, col_1_value..col_4_value)` — Rows of custom plugin tables shown on a Player page.
10. **`plan_extension_server_table_values`**: `(id, table_id, col_1_value..col_5_value)` — Rows of custom plugin tables shown on a Server page.

---

## 5. Embedded Webserver, Security, and Complete REST API Specification

Plan embeds a standalone HTTP/HTTPS server (default port `8804`) that serves its React dashboard (`Plan/react/dashboard/build`) and responds to JSON data queries.

### 5.1 Webserver Architecture & Caching Layers

To prevent expensive SQL aggregations from running on every page refresh, Plan implements a multi-tier response cache:

* **Reduced Refresh Barrier (`15 SECONDS`):** If a client requests a JSON endpoint that was generated less than 15 seconds ago, the cached byte payload is returned immediately without querying the database.
* **In-Memory Cache (`5 MINUTES`):** Frequently accessed JSON payloads (like `/v1/serverOverview` or `/v1/graph?type=playersOnline`) are held in an in-memory LRU/TTL cache.
* **Disk Cache (`2 DAYS`):** Serialized JSON responses can be written to disk for offline viewing or static exporting.
* **Query Results Cache (`7 DAYS`):** Dynamic cohort queries generated via `/v1/query` are assigned a unique timestamp/hash token and cached for 7 days so admins can share result links with staff.

### 5.2 Authentication, TLS & RBAC Permission Tree

* **TLS / HTTPS:** Supports PKCS12 (`.p12`), JKS (`.jks`), or PEM certificate chains. When HTTPS is active (or behind a reverse proxy with `Use_X-Forwarded-For_Header: true`), authentication is enforced unless `Disable_authentication: true`.
* **Registration Handshake Flow:**
1. A user opens `/register` in their browser, enters a desired username and password, and submits `POST /auth/register`.
2. Plan hashes the password with **BCrypt** (or salted PBKDF2), stores a temporary pending registration in memory with a random 12-character hexadecimal `code`, and returns the code to the browser.
3. The player runs `/plan register --code <code>` in-game (which links their Minecraft `UUID` to the web account and assigns the default `player` web group) OR an admin runs `/plan register --code <code>` in the server console (which assigns the `admin` web group).


* **Complete Web Permission Hierarchy:**
* `page.network` (and child nodes `page.network.overview`, `page.network.server.list`, `page.network.playerbase`, `page.network.sessions`, `page.network.join.addresses`, `page.network.retention`, `page.network.geolocations`, `page.network.plugins`)
* `page.server` (and child nodes `page.server.overview`, `page.server.online.activity`, `page.server.sessions`, `page.server.pvppve`, `page.server.playerbase`, `page.server.retention`, `page.server.players`, `page.server.geolocations`, `page.server.performance`, `page.server.plugin.history`, `page.server.plugins`)
* `page.player.other` (Can view any player's `/player/{uuid}` profile)
* `page.player.self` (Can only view their own linked `/player/{uuid}` profile)
* `access.player.list` (Can view `/players` directory)
* `access.query` (Can use `/query` builder)
* `access.errors` (Can view `/errors` diagnostics log)
* `access.docs` (Can view `/docs` OpenAPI/Swagger documentation)
* `access.theme.editor` (Can use `/theme-editor`)
* `manage.groups` (Can modify RBAC permission matrices in the web UI)
* `manage.themes` (Can save/delete custom JSON themes on the server)



---

### 5.3 Exhaustive REST API Endpoints (`/v1/*`)

Every endpoint accepts optional `?timestamp=<epoch_ms>` for ETag/304 caching and returns JSON with UTF-8 encoding.

#### A. Server-Scoped Endpoints

| HTTP Method & Endpoint | Query Parameters | Returned JSON Payload Structure |
| --- | --- | --- |
| `GET /v1/serverOverview` | `server={uuid_or_name}` | `{ "last_7_days": { unique_players, new_players, new_players_retention, new_players_retention_perc, avg_tps, low_tps_spikes, downtime }, "numbers": { total_players, regular_players, online_players, current_uptime, best_peak_players, best_peak_date, last_peak_players, last_peak_date }, "weeks": { start, end, unique_before, unique_after, unique_trend, new_before, new_after, ... } }` |
| `GET /v1/onlineOverview` | `server={uuid_or_name}` | `{ "numbers": { unique_players_30d, unique_players_7d, unique_players_24h, unique_players_30d_avg, new_players_30d, new_players_7d, new_players_24h, new_players_retention_30d, ... }, "insights": { first_session_avg_length, first_session_median_length, lone_joins, lone_new_joins, players_online_on_first_join_avg } }` |
| `GET /v1/sessionsOverview` | `server={uuid_or_name}` | `{ "insights": { most_active_gamemode, most_active_gamemode_perc, server_occupied, server_occupied_perc, total_playtime, afk_time, afk_time_perc } }` |
| `GET /v1/playerVersus` | `server={uuid_or_name}` | `{ "numbers": { player_kills_total, player_kills_30d, player_kills_7d, kdr_total, kdr_30d, kdr_7d, mob_kills_total, mob_kills_30d, mob_kills_7d, mob_deaths_total, mob_kdr_total, ... }, "insights": { deadliest_pvp_weapon, second_deadliest_pvp_weapon, third_deadliest_pvp_weapon } }` |
| `GET /v1/playerbaseOverview` | `server={uuid_or_name}` | `{ "trends": { total_players_now, total_players_then, total_players_trend, regular_players_now, regular_players_then, regular_players_trend, playtime_avg_now, playtime_avg_then, afk_then, afk_now, regular_playtime_avg_now, regular_session_avg_now }, "insights": { avg_activity_index, avg_global_playtime } }` |
| `GET /v1/performanceOverview` | `server={uuid_or_name}` | `{ "numbers": { low_tps_spikes_30d, low_tps_spikes_7d, low_tps_spikes_24h, server_downtime_30d, server_downtime_7d, server_downtime_24h, avg_server_uptime_30d, players_30d, players_7d, players_24h, tps_30d, tps_7d, tps_24h, cpu_30d, cpu_7d, cpu_24h, ram_30d, ram_7d, ram_24h, entities_30d, chunks_30d, max_disk_30d, min_disk_30d, mspt_average_30d, mspt_95th_30d }, "insights": { low_tps_players, low_tps_chunks, low_tps_entities, low_tps_tps } }` |
| `GET /v1/sessions` | `server={uuid_or_name}` | `{ "sessions": [ { player_name, player_uuid, server_name, server_uuid, start, end, length, afk_time, mob_kills, deaths, first_session, join_address, world_series: [...], gm_series: {...}, player_kills: [...] } ], "most_recent_sessions": [...] }` |
| `GET /v1/kills` | `server={uuid_or_name}` | `{ "player_kills": [ { killer_name, killer_uuid, victim_name, victim_uuid, server_name, weapon, date, time_ago } ] }` |
| `GET /v1/pingTable` | `server={uuid_or_name}` | `{ "ping_table": [ { country, avg_ping, min_ping, max_ping } ] }` |
| `GET /v1/players` | `server={uuid_or_name}` | `{ "data": [ { name, uuid, activity_index, activity_group, active_playtime, sessions, registered, last_seen, geolocation, ping_avg, ping_min, ping_max, nick, extension_values: {...} } ] }` |
| `GET /v1/playersTable` | `server={uuid_or_name}` | Paginated / formatted variant of `/v1/players` with DataExtension columns merged (`show_in_players_table = true`). |
| `GET /v1/joinAddresses` | `server={uuid_or_name}` | `{ "join_addresses": [ { join_address, count } ] }` |
| `GET /v1/retention` | `server={uuid_or_name}` | `{ "new_players": [ { uuid, name, registered, last_seen, playtime, active_playtime, session_count, activity_index, join_address } ] }` |
| `GET /v1/extensionData` | `server={uuid_or_name}` | `{ "extensions": [ { plugin_name, icon, last_updated, tabs: [ { tab_name, values: [...], tables: [...] } ] } ] }` |

#### B. Graph & Time-Series Endpoint (`GET /v1/graph`)

The `/v1/graph` endpoint is polymorphic and switches its SQL query and mathematical processor based on `?type={graphType}&server={uuid}`:

* `type=playersOnline` $\longrightarrow$ Array of `[epoch_ms, players_online]`
* `type=tps` $\longrightarrow$ Arrays of `[epoch_ms, tps]`, `[epoch_ms, mspt_avg]`, `[epoch_ms, mspt_95th]`
* `type=cpuRam` $\longrightarrow$ Arrays of `[epoch_ms, cpu_usage]`, `[epoch_ms, ram_usage]`
* `type=worldLoad` $\longrightarrow$ Arrays of `[epoch_ms, entities]`, `[epoch_ms, chunks_loaded]`
* `type=disk` $\longrightarrow$ Array of `[epoch_ms, free_disk_space]`
* `type=ping` $\longrightarrow$ Arrays of `[epoch_ms, avg_ping]`, `[epoch_ms, max_ping]`, `[epoch_ms, min_ping]`
* `type=uniqueAndNew` $\longrightarrow$ Daily time-series of `[day_epoch_ms, unique_players, new_players]`
* `type=hourlyUniqueAndNew` $\longrightarrow$ Hourly time-series of `[hour_epoch_ms, unique_players, new_players]`
* `type=serverCalendar` $\longrightarrow$ FullCalendar event array of daily unique/new counts, playtime, and sessions
* `type=worldPie` $\longrightarrow$ `{ "world_series": [{ name, y, color }], "gm_series": [{ name, data: [...] }] }`
* `type=activity` $\longrightarrow$ `{ "activity_pie_series": [...], "activity_series": { "Very Active": [[date, count]], ... } }`
* `type=geolocation` $\longrightarrow$ `{ "geolocation_series": [{ code: "AUS", value: 42, name: "Australia" }], "geolocation_bar_series": [...] }`
* `type=punchCard` $\longrightarrow$ Array of `{ x: hour_ms, y: day_index, z: bubble_radius, marker: { radius } }`
* `type=joinAddressesByDay` $\longrightarrow$ Stacked time-series of player joins grouped by virtual host per day
* `type=joinAddressesPie` $\longrightarrow$ Pie chart breakdown of latest join addresses

#### C. Player-Scoped & Network-Scoped Endpoints

* `GET /v1/player?player={uuid_or_name}`: Returns the complete JSON profile of a single player:
* `info`: `{ name, uuid, online, operator, banned, kick_count, player_kill_count, mob_kill_count, death_count, activity_index, activity_group, active_playtime, total_playtime, afk_time, session_count, longest_session_length, session_median, registered, last_seen, favorite_server, avg_ping, min_ping, max_ping }`
* `nicknames`: Array of `{ nickname, server, last_used }`
* `connections`: Array of `{ geolocation, date }`
* `sessions`: Last $N$ sessions with full per-world/per-gamemode breakdown
* `world_pie_series` & `gm_series`: Global and per-server world/gamemode playtime breakdown
* `server_pie_series`: Playtime percentage distribution across network servers
* `punchcard_series`: Personal $7 \times 24$ activity habit matrix
* `calendar_series`: Personal daily session calendar
* `player_kills` & `player_deaths`: Full PvP kill and death logs
* `extensions`: Per-server `DataExtension` values and tables for this specific player


* `GET /v1/network/overview`, `GET /v1/network/servers`, `GET /v1/network/playerbaseOverview`, `GET /v1/network/sessionsOverview`, `GET /v1/network/pingTable`: Network-wide equivalents that aggregate across all game servers while using proxy sessions (or unioned game server sessions) for network online counts.

#### D. Dynamic Cohort Query Engine (`POST /v1/query` & `GET /v1/filters`)

The `/query` page allows staff to build multi-filter boolean queries over the playerbase and inspect how that cohort behaved between two dates (`viewAfterDate`/`viewBeforeDate`):

* `GET /v1/filters`: Returns all available filter types and their valid options:
* `registeredBetween` (`after`, `before`)
* `lastSeenBetween` (`after`, `before`)
* `playedBetween` (`after`, `before`, `servers: [...]`)
* `playedOnServer` (`servers: [...]`)
* `activityIndexNow` (`above`, `below` or `groups: ["Very Active", "Active"]`)
* `activityIndexOn` (`date`, `groups: [...]`)
* `banned` (`["Banned", "Not banned"]`)
* `operators` (`["Operators", "Non-operators"]`)
* `joinAddresses` (`["mc.example.com", ...]`)
* `geolocations` (`["United States", "Australia", ...]`)
* `pluginsBooleanGroups` (Dynamic filter on any `DataExtension` boolean or group provider!)
* `pluginNumbers` (Dynamic range filter `[min, max]` on any `DataExtension` number/double/percentage provider!)


* `POST /v1/query`: Executes the intersection ($\bigcap$) of all selected filters to produce a set of matching `user_id`s, then computes a mini-dashboard specifically for that cohort (Activity Index progression over time, Retention comparison, Session summary, and a downloadable Players table).

---

## 6. Frontend Architecture & Customization System

Plan's web frontend is a modern **React 18 + Bootstrap 5 + Highcharts / Chart.js + DataTables** Single Page Application located in `Plan/react/dashboard`.

### 6.1 Pages & Views

1. **Server Page (`/server/{serverNameOrUuid}`):**
* *Server Overview:* 7-day summary, Online Activity numbers, Server-to-Server comparison, 30-day trends.
* *Online Activity:* Players Online & New/Unique charts, Day-by-Day & Hour-by-Hour breakdowns, Server Calendar, PunchCard, Assembled Join Time insights.
* *Sessions:* Expandable list of the 50 most recent sessions, World Playtime Pie + Gamemode Drilldown, Server Occupation insights.
* *PvP & PvE:* Kill/Death numbers, Top 3 Weapons, Recent PvP Kills table, Mob KDR.
* *Playerbase Overview:* 30-day Activity Index Stacked Area graph, Current Activity Pie, Regular vs Total trends.
* *Player Retention:* Interactive multi-variable scatter/cohort explorer comparing `Registered Date`, `Playtime`, `Activity Index`, `Join Address`, and `Last Seen`.
* *Player List:* Searchable, sortable DataTables grid of up to 25,000 players with dynamically injected plugin columns.
* *Geolocations:* Interactive Vector World Map choropleth of player origins + Country Ping table.
* *Performance:* Synchronized multi-axis Highcharts time-series (`Players Online`, `TPS`, `MSPT Avg & 95th`, `CPU %`, `RAM MB`, `Entities`, `Loaded Chunks`, `Free Disk Space`) + Low TPS Spike correlation cards.
* *Plugin History & Third-Party Plugin Tabs:* Custom tabs dynamically created from `DataExtension` registrations.


2. **Network Page (`/network`):** Network Overview, Servers Comparison List, Network Sessions, Network Playerbase, Network Retention, Join Addresses, Geolocations, Network Plugins.
3. **Players Directory (`/players`):** Global player directory across all servers.
4. **Player Profile (`/player/{uuid}`):** Overview card, Nicknames table, Connection/Country history, Personal PunchCard, Personal Session Calendar, World/Gamemode Pie, Server Preference Pie, Recent Sessions, PvP Kills/Deaths, and per-server Plugin tabs.
5. **Query Page (`/query`):** Interactive cohort builder and result viewer.
6. **Theme Editor (`/theme-editor`) & Manage Access (`/manage`):** Visual CSS/Chart color editor and RBAC group/permission manager.

### 6.2 Theme System & Static HTML Export

* **JSON Theme System:** Themes (e.g., `default.json`, `night.json`, `dracula.json`, `cyberpunk.json`) define CSS variables (`--color-primary`, `--color-bg`, `--color-card`), Highcharts palette arrays, pie chart color maps, and night-mode overrides. They are served via `GET /v1/theme?name={theme}`.
* **Static HTML & JSON Exporter (`Export` system):** For servers that do not wish to expose Plan's port (`8804`) to the public internet, Plan can periodically dump pre-rendered JSON files and the React bundle into a local folder (`Export.Export_path`). A standard Nginx/Apache server can then host those static files with zero backend database access.

---

## 7. In-Game Commands, Permissions & PlaceholderAPI

### 7.1 Complete Command Tree & Permission Matrix

| Command & Syntax | Aliases | Permission Node | Default | Functional Behavior |
| --- | --- | --- | --- | --- |
| `/plan` (or `/planproxy`) | `planbungee`, `planvelocity` | `plan.command` | `op` | Displays interactive help menu of permitted subcommands. |
| `/plan server [server]` | `analyze`, `a`, `analysis` | `plan.server` | `op` | Generates a clickable chat URL to the `/server` page. |
| `/plan servers` | `serverlist`, `sl`, `ls` | `plan.servers` | `op` | Lists all registered servers in the database (ID, UUID, Name, Version). |
| `/plan network` | `netw` | `plan.network` | `op` | Generates a clickable chat URL to the `/network` page. |
| `/plan player [player]` | `inspect` | `plan.player.self` / `plan.player.other` | `true` / `op` | Generates a clickable chat URL to the player's `/player/{uuid}` profile. |
| `/plan players` | `pl`, `playerlist`, `list` | `plan.player.other` | `op` | Generates a clickable chat URL to the `/players` directory. |
| `/plan ingame [player]` | `qinspect` | `plan.ingame.self` / `plan.ingame.other` | `true` / `op` | Prints a formatted summary of the player's stats directly in Minecraft chat (Playtime, Active/AFK, Activity Index, Last Seen, Geolocation, KDR, Kick count). |
| `/plan search <query>` | — | `plan.search` | `op` | Performs a wildcard `LIKE %query%` search on `plan_users` and returns matching names. |
| `/plan json [player]` | — | `plan.json.self` / `plan.json.other` | `true` / `op` | Provides a direct download link to the raw JSON dump of the player's data (GDPR Right of Access). |
| `/plan register [--code <code>]` | — | `plan.register.self` / `plan.register.other` | `true` / `op` | Links a pending web registration code to the command sender (or grants admin web group if executed from console). |
| `/plan unregister [username]` | — | `plan.unregister.self` / `plan.unregister.other` | `true` / `op` | Deletes a web user account from `plan_security`. |
| `/plan setgroup <user> <group>` | — | `plan.setgroup.other` | `op` | Updates a web user's RBAC `group_id` in `plan_security`. |
| `/plan users` | `webusers` | `plan.users` | `op` | Lists all registered web users and their assigned groups. |
| `/plan logout <user>` | — | `plan.logout.other` | `op` | Invalidates all active session cookies for `<user>` in `plan_cookies`. |
| `/plan info` | — | `plan.info` | `op` | Displays Plan version, active database type/status, webserver port, and update status. |
| `/plan reload` | — | `plan.reload` | `op` | Full hot-reload of config, database pool, webserver, and extension listeners. |
| `/plan disable [kickcount]` | — | `plan.disable` | `op` | Shuts down Plan (or temporarily disables kick counting before a server restart). |
| `/plan accept` / `/plan cancel` | `yes`, `y` / `no`, `n` | *(None)* | `true` | Confirms or aborts destructive `/plan db` operations. |
| `/plan export [directory]` | — | `plan.data.export` | `op` | Triggers an immediate static JSON/HTML export of all server/player pages. |
| `/plan import` | — | `plan.data.import` | `op` | Imports legacy or offline player data into the Plan database. |
| `/plan db backup <db_type>` | — | `plan.data.backup` | `op` | Clones the entire active database into a timestamped SQLite `.db` backup file. |
| `/plan db restore <file> <db>` | — | `plan.data.restore` | `op` | Restores a SQLite backup file into the target database (`MySQL`, `SQLite`, `H2`). |
| `/plan db move <from> <to>` | — | `plan.data.move` | `op` | Migrates all schema tables and records between two database backends (e.g., `SQLite` $\rightarrow$ `MySQL`). |
| `/plan db merge <from> <to>` | — | `plan.data.merge` | `op` | Merges two separate server databases into a single unified database. |
| `/plan db hotswap <db_type>` | — | `plan.data.hotswap` | `op` | Changes the active database config to `<db_type>` and reloads Plan immediately. |
| `/plan db clear <db_type>` | — | `plan.data.clear` | `op` | Drops/truncates all Plan data in `<db_type>` (requires `/plan accept` confirmation). |
| `/plan db remove <player>` | — | `plan.data.remove.player` | `op` | Completely purges all records of `<player>` across all tables (GDPR Right to Erasure). |
| `/plan db uninstalled [server]` | — | `plan.data.remove.server` | `op` | Marks a decommissioned server as `is_installed = 0` so it no longer clutters active network views. |
| `/plan db optimize` | — | `plan.data.base` | `op` | Executes `VACUUM` / `OPTIMIZE TABLE` on the underlying SQL database. |

**Special Passive Permission Nodes:**

* `plan.ignore.commanduse` (Default: `false`): Commands executed by players with this permission are never logged in `plan_commandusages`.
* `plan.ignore.afk` (Default: `false`): Players with this permission are never marked AFK by the `AFKTracker`.

---

### 7.2 Complete PlaceholderAPI (`%plan_*%`) Specification

Plan exposes over 90 placeholders for in-game scoreboards, tablists, and holograms. Time suffixes `_day`, `_week`, and `_month` filter over the last `24 hours`, `7 days`, and `30 days` respectively. Adding `_raw` to any duration placeholder returns unformatted **milliseconds**.

#### Server & Network Placeholders

* **Identity:** `%plan_server_name%`, `%plan_server_uuid%`
* **Player Counts:** `%plan_players_total%`, `%plan_players_new_day%`, `%plan_players_new_week%`, `%plan_players_new_month%`, `%plan_players_unique_day%`, `%plan_players_unique_week%`, `%plan_players_unique_month%`, `%plan_players_online%`, `%plan_players_max_peak_alltime%`, `%plan_players_max_peak_date_alltime%`, `%plan_players_max_peak_recent%`, `%plan_players_max_peak_date_recent%`, `%plan_players_regular%`
* **Playtime & Sessions:** `%plan_sessions_playtime_total%`, `%plan_sessions_playtime_day%`, `%plan_sessions_playtime_week%`, `%plan_sessions_playtime_month%`, `%plan_sessions_active_time_total%`, `%plan_sessions_afk_time_total%`, `%plan_sessions_count_total%`, `%plan_sessions_count_day%`, `%plan_sessions_count_week%`, `%plan_sessions_count_month%`, `%plan_sessions_average_length_day%`, `%plan_sessions_average_length_week%`, `%plan_sessions_average_length_month%`, `%plan_sessions_average_unique_players_day%`, `%plan_sessions_average_unique_players_week%`, `%plan_sessions_average_unique_players_month%`
* **Combat:** `%plan_sessions_player_kills_total%`, `%plan_sessions_player_kills_day%`, `%plan_sessions_player_kills_week%`, `%plan_sessions_player_kills_month%`, `%plan_sessions_mob_kills_total%`, `%plan_sessions_mob_kills_day%`, `%plan_sessions_mob_kills_week%`, `%plan_sessions_mob_kills_month%`, `%plan_sessions_deaths_total%`, `%plan_sessions_deaths_day%`, `%plan_sessions_deaths_week%`, `%plan_sessions_deaths_month%`
* **Performance & Ping:** `%plan_tps_day%`, `%plan_tps_week%`, `%plan_tps_month%`, `%plan_tps_spikes_day%`, `%plan_tps_spikes_week%`, `%plan_tps_spikes_month%`, `%plan_ping_average_day%`, `%plan_ping_average_week%`, `%plan_ping_average_month%`, `%plan_ping_min_day%`, `%plan_ping_max_day%`, `%plan_server_time_zone%`, `%plan_server_time_zone_offset%`
* **Top Playtime Leaderboard (`1` to `10`):**
* `%plan_top_player_playtime_<total/day/week/month>_<active/afk>_<1..10>%` (Returns player name)
* `%plan_top_amount_playtime_<total/day/week/month>_<active/afk>_<1..10>%` (Returns formatted time)



#### Player-Specific Placeholders

* **Playtime (`_raw` supported on all):** `%plan_player_time_total%`, `%plan_player_time_day%`, `%plan_player_time_week%`, `%plan_player_time_month%`, `%plan_player_time_active_total%`, `%plan_player_time_active_day%`, `%plan_player_time_active_week%`, `%plan_player_time_active_month%`, `%plan_player_time_afk_total%`, `%plan_player_time_afk_day%`, `%plan_player_time_afk_week%`, `%plan_player_time_afk_month%`
* **Sessions:** `%plan_player_session_length%` (current live session), `%plan_player_session_length_average%`, `%plan_player_session_length_Longest%`, `%plan_player_sessions_count%`, `%plan_player_registered%`, `%plan_player_lastseen%`, `%plan_player_is_banned%`, `%plan_player_is_operator%`, `%plan_player_kicked_count%`
* **Activity & Connection:** `%plan_player_activity_index%`, `%plan_player_activity_group%`, `%plan_player_geolocation%`, `%plan_player_ping_average%`, `%plan_player_ping_max%`, `%plan_player_ping_min%`, `%plan_player_favorite_server%`
* **Combat:** `%plan_player_player_kills%`, `%plan_player_mob_kills%`, `%plan_player_deaths%`, `%plan_player_kdr%`, `%plan_player_mob_kdr%`, `%plan_player_player_caused_deaths%`, `%plan_player_recent_kill_1%`..`%plan_player_recent_kill_10%`, `%plan_player_recent_kill_weapon_1%`..`%plan_player_recent_kill_weapon_10%`

---

## 8. Public Developer APIs (`APIv5`)

Other plugins integrate with Plan through four singleton services in `com.djrapitops.plan`:

1. **`CapabilityService`:** Allows callers to query `hasCapability("DATA_EXTENSION_VALUES")` and register reload listeners via `registerEnableListener(isEnabled -> ...)`.
2. **`ExtensionService` (`DataExtension` API):**
* In Java, plugins annotate a class with `@PluginInfo(name = "Vault", iconName = "money-bill-wave", iconFamily = Family.SOLID, color = Color.GREEN)` and annotate methods with `@BooleanProvider`, `@NumberProvider(format = FormatType.DATE_YEAR | FormatType.TIME_MILLISECONDS | FormatType.NONE)`, `@DoubleProvider`, `@PercentageProvider`, `@StringProvider(playerName = true)`, `@GroupProvider`, `@TableProvider`, and `@Conditional("hasTown")`.
* **CallEvents:** Extensions declare when Plan should invoke their providers: `PLAYER_JOIN`, `PLAYER_LEAVE`, `SERVER_EXTENSION_REGISTER`, `SERVER_PERIODICAL`, or `MANUAL` (triggered on-demand via `ExtensionService.getInstance().updatePlayerValues(uuid, CallEvents.MANUAL)`).


3. **`QueryService`:** Allows third-party plugins to execute custom read/write SQL queries directly against Plan's database connection pool (`queryService.query(sql, preparedStatement -> ...)`), as well as subscribe to player data removal events (`subscribeDataClearEvent`).
4. **`ResolverService` (`PageExtension` API):** Allows third-party plugins to register custom HTTP endpoints and inject custom JavaScript/CSS into the Plan web server.

---

## 9. Step-by-Step Developer Porting Blueprint for PumpkinMC

When building **`pumpkin-plan`** for PumpkinMC, here is the exact engineering blueprint to translate every Java subsystem into idiomatic, lock-free Rust.

---

### 9.1 Rust Crate & Dependency Architecture

Structure your workspace into three modular crates so the core analytics engine is decoupled from the Minecraft version protocol:

```toml
# Cargo.toml (Workspace Root)
[workspace]
members = [
    "pumpkin-plan-core",    # Database (SQLx), ActivityIndex math, Session state, Geolocation (.mmdb)
    "pumpkin-plan-web",     # Axum HTTP/HTTPS server, REST /v1/* handlers, RBAC auth, React SPA static assets
    "pumpkin-plan-plugin",  # PumpkinMC Plugin trait, EventHandlers, Tick sampler, /plan commands
]

[workspace.dependencies]
tokio = { version = "1.40", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "sqlite", "mysql", "postgres", "macros"] }
axum = { version = "0.8", features = ["ws", "multipart", "macros"] }
tower-http = { version = "0.6", features = ["fs", "cors", "compression-gzip", "trace"] }
dashmap = "6.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.10", features = ["v4", "serde"] }
maxminddb = "0.24"
bcrypt = "0.15"
chrono = { version = "0.4", features = ["serde"] }
rust-embed = "8.5" # Embeds Plan's compiled React dashboard right inside the binary!

```

---

### 9.2 Mapping Plan's Listeners to PumpkinMC's Event System

PumpkinMC distinguishes between **Blocking Events** (sequential, can cancel/modify) and **Non-Blocking Events** (concurrent, zero main-thread stall). Because Plan is strictly an **observational** analytics plugin, **100% of your event handlers in PumpkinMC should be registered as Non-Blocking (`blocking = false`)**.

```rust
use pumpkin_api_macros::plugin_impl;
use pumpkin_plugin_api::{Context, EventPriority, Plugin, PluginMetadata};
use std::sync::Arc;

pub struct PumpkinPlanPlugin {
    state: Arc<PlanEngineState>,
}

impl Plugin for PumpkinPlanPlugin {
    fn new() -> Self {
        let state = Arc::new(PlanEngineState::initialize_from_config());
        Self { state }
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "Plan".into(),
            version: "5.7.0-pumpkin".into(),
            authors: vec!["AuroraLS3".into(), "PumpkinMC Port".into()],
            description: "Player Analytics for PumpkinMC".into(),
        }
    }

    fn on_load(&mut self, context: Context) -> pumpkin_plugin_api::Result<()> {
        // 1. Register Non-Blocking Event Handlers (Zero Server Tick Impact!)
        context.register_event_handler(
            JoinHandler::new(self.state.clone()),
            EventPriority::Monitor,
            false, // non-blocking!
        )?;
        context.register_event_handler(
            LeaveHandler::new(self.state.clone()),
            EventPriority::Monitor,
            false,
        )?;
        context.register_event_handler(
            MoveAndAfkHandler::new(self.state.clone()),
            EventPriority::Monitor,
            false,
        )?;
        context.register_event_handler(
            WorldAndGamemodeHandler::new(self.state.clone()),
            EventPriority::Monitor,
            false,
        )?;
        context.register_event_handler(
            CombatAndDeathHandler::new(self.state.clone()),
            EventPriority::Monitor,
            false,
        )?;
        context.register_event_handler(
            CommandAndChatHandler::new(self.state.clone()),
            EventPriority::Monitor,
            false,
        )?;

        // 2. Spawn Background Tokio Tasks (TPS Sampler, Ping Sampler, DB Writer, Axum Webserver)
        self.state.spawn_background_tasks();

        Ok(())
    }

    fn on_unload(&mut self, _context: Context) -> pumpkin_plugin_api::Result<()> {
        // Flush all active sessions in DashMap to SQL before shutdown (up to 20s timeout)
        self.state.flush_all_active_sessions_blocking();
        Ok(())
    }
}

```

---

### 9.3 Implementing the Core Engines in Rust

#### 1. The Lock-Free Session & AFK Engine (`pumpkin-plan-core/src/session.rs`)

Use `DashMap<Uuid, ActiveSession>` and `DashMap<Uuid, i64>` (`last_movement`) so concurrent non-blocking movement/chat events never contend on a global `Mutex`:

```rust
use dashmap::DashMap;
use uuid::Uuid;

pub struct SessionManager {
    pub active_sessions: DashMap<Uuid, ActiveSession>,
    pub last_movement: DashMap<Uuid, i64>,
    pub afk_threshold_ms: i64, // Default: 180_000 (3 minutes)
    pub db_tx: tokio::sync::mpsc::UnboundedSender<DbTransaction>,
}

impl SessionManager {
    pub fn on_player_activity(&self, uuid: Uuid, now_ms: i64, ignores_afk: bool) {
        if ignores_afk {
            self.last_movement.insert(uuid, -1);
            return;
        }
        let mut last = self.last_movement.entry(uuid).or_insert(now_ms);
        if *last != -1 {
            let delta = now_ms - *last;
            if delta > self.afk_threshold_ms {
                if let Some(mut session) = self.active_sessions.get_mut(&uuid) {
                    session.afk_time += delta;
                }
            }
        }
        *last = now_ms;
    }

    pub fn on_player_quit(&self, uuid: Uuid, quit_ms: i64) {
        self.on_player_activity(uuid, quit_ms, false);
        self.last_movement.remove(&uuid);
        if let Some((_, mut session)) = self.active_sessions.remove(&uuid) {
            session.session_end = Some(quit_ms);
            session.world_times.finalize(quit_ms);
            // Send completed session to async SQL writer queue
            let _ = self.db_tx.send(DbTransaction::StoreCompletedSession(session));
        }
    }
}

```

#### 2. The Activity Index Calculator (`pumpkin-plan-core/src/activity_index.rs`)

Implement the exact asymptotic formula so the React frontend charts match Java Plan down to the second decimal place:

```rust
use std::f64::consts::FRAC_PI_2;

pub struct ActivityIndex {
    pub value: f64,
    pub group: &'static str,
}

impl ActivityIndex {
    pub fn calculate(playtime_w1_ms: u64, playtime_w2_ms: u64, playtime_w3_ms: u64, threshold_ms: f64) -> Self {
        let calc_week = |playtime: u64| -> f64 {
            1.0 - (1.0 / (FRAC_PI_2 * (playtime as f64 / threshold_ms) + 1.0))
        };

        let y1 = calc_week(playtime_w1_ms);
        let y2 = calc_week(playtime_w2_ms);
        let y3 = calc_week(playtime_w3_ms);

        let value = 5.0 * ((3.0 * y1 + 2.0 * y2 + 1.0 * y3) / 6.0);
        let group = match value {
            v if v >= 3.75 => "Very Active",
            v if v >= 3.00 => "Active",
            v if v >= 2.00 => "Regular",
            v if v >= 1.00 => "Irregular",
            _ => "Inactive",
        };

        Self { value, group }
    }
}

```

#### 3. Reusing Plan's React Dashboard Without Rewriting the UI

You **do not** need to rewrite Plan's 50,000+ lines of React/Highcharts frontend code. Plan's frontend is completely decoupled from Java and communicates strictly via the `/v1/*` JSON REST contracts documented in Section 5.3:

1. Run `npm install && npm run build` inside `Plan/react/dashboard` from the upstream Plan repository.
2. Embed the resulting `build/` directory into your `pumpkin-plan-web` crate using `rust-embed` (or serve it from `/plugins/Plan/public_html` via `tower_http::services::ServeDir`).
3. Mount your **Axum** router on port `8804` with the `/v1/*` routes returning `serde_json` structs matching Section 5.3.

#### 4. Designing a Rust-Native `DataExtension` Trait for PumpkinMC Plugins

To allow other Rust plugins on PumpkinMC (e.g., economy, permissions, or minigame plugins) to display their tabs and tables inside Plan, expose a trait in a lightweight `pumpkin-plan-api` crate:

```rust
pub trait PlanDataExtension: Send + Sync {
    fn plugin_name(&self) -> &'static str;
    fn plugin_icon(&self) -> ExtensionIcon;
    fn call_events(&self) -> &[CallEvent];

    fn provide_player_data(&self, player_uuid: Uuid) -> Vec<ExtensionValue> {
        Vec::new()
    }

    fn provide_server_data(&self) -> Vec<ExtensionValue> {
        Vec::new()
    }
}

```

---

### 9.4 Alternative Path: Running Unmodified Java `Plan.jar` via PatchBukkit

If you prefer to run the upstream Java `Plan.jar` inside PumpkinMC's **PatchBukkit** JVM bridge rather than compiling a native Rust port, verify the following FFI/Protobuf hooks in `PatchBukkit`:

1. **`Server.getTPS()` & `Server.getTickTimes()`:** Ensure `rust/src/java/native_callbacks` exposes PumpkinMC's real-time tick duration ring buffer to `PatchBukkitServer.java` so `plan_tps` records accurate TPS and MSPT (95th percentile).
2. **`PlayerLoginEvent.getHostname()` & `Player.getAddress()`:** Ensure the protobuf definition for `PlayerLoginEvent` passes both the client's remote socket IP (`InetSocketAddress`) and the raw handshake virtual host string (`server_address`) across the FFI bridge so Plan's Geolocation and Join Address trackers function.
3. **`Player.getPing()`:** Expose the player's KeepAlive RTT latency from PumpkinMC's network client struct to `PatchBukkitPlayer.getPing()`.

---