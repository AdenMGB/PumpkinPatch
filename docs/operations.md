# Pumpkin Patch operations

## Health

The desktop app probes Velocity (ping), backend TCP ports, RCON, and Patch Plan export freshness. Health snapshots are stored in SQLite for library sparklines.

## Ports

Before start, Patch bind-checks the hub port and every server `game_port` on the configured bind host. Use **Create network** to preview the proposed port map.

## Process supervision

Child processes are watched; unexpected exits emit `network-process-exited`, mark the network **error**, and optionally auto-restart (max 5 attempts per crash cycle).

Shutdown order: Velocity `shutdown` → RCON `stop` on backends → lobby → kill remaining processes.

## Backup and restore

**Backup network** writes `backups/{network_id}.zip` under the Patch data directory (manifest + full data tree). **Import backup** creates a new network id and restores files.

## Java

Settings require Java `java_min_major` (default 21). Start is blocked until validation passes; use **Validate Java** in Settings.
