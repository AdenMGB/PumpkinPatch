use crate::error::Result;
use std::path::Path;

/// RCON listen port for a Pumpkin instance (loopback only).
pub fn rcon_port_for_game_port(game_port: u16) -> u16 {
    game_port.saturating_add(2000)
}

/// Deterministic RCON password for managed instances (not stored in the DB).
pub fn rcon_password_for_server(forwarding_secret: &str, game_port: u16) -> String {
    let mut hash = 0u64;
    for (i, b) in forwarding_secret.bytes().enumerate() {
        hash ^= (b as u64) << ((i % 8) * 8);
        hash = hash.rotate_left(5);
    }
    hash ^= (game_port as u64) << 32;
    hash ^= forwarding_secret.len() as u64;
    format!("pp{:016x}", hash)
}

/// Pumpkin loads only `pumpkin.toml` (flattened config: `[networking.java]`, not `[advanced.networking.java]`).
pub fn sync_server_instance_config(
    instance_path: &Path,
    game_port: u16,
    forwarding_secret: &str,
) -> Result<()> {
    std::fs::create_dir_all(instance_path)?;
    let bind = format!("127.0.0.1:{game_port}");
    let rcon_port = rcon_port_for_game_port(game_port);
    let rcon_password = rcon_password_for_server(forwarding_secret, game_port);
    let rcon_addr = format!("127.0.0.1:{rcon_port}");

    let patch = format!(
        r#"# Managed by Pumpkin Patch — reference copy (Pumpkin reads pumpkin.toml)
[networking.java]
address = "{bind}"
online_mode = false
encryption = false

[networking.proxy]
enabled = true

[networking.proxy.velocity]
enabled = true
secret = "{secret}"

[networking.bedrock]
enabled = false

[networking.lan_broadcast]
enabled = false

[networking.rcon]
enabled = true
address = "{rcon_addr}"
password = "{rcon_password}"
"#,
        bind = bind,
        secret = forwarding_secret,
        rcon_addr = rcon_addr,
        rcon_password = rcon_password,
    );
    std::fs::write(instance_path.join("pumpkin.patch.toml"), &patch)?;

    let pumpkin_path = instance_path.join("pumpkin.toml");
    if pumpkin_path.exists() {
        let text = std::fs::read_to_string(&pumpkin_path)?;
        let mut root: toml::Value = toml::from_str(&text)?;
        apply_network_patch(&mut root, &bind, forwarding_secret, game_port);
        std::fs::write(pumpkin_path, toml::to_string_pretty(&root)?)?;
    } else {
        std::fs::write(&pumpkin_path, &patch)?;
    }
    Ok(())
}

fn apply_network_patch(root: &mut toml::Value, bind: &str, secret: &str, game_port: u16) {
    set_nested_bool(root, &["networking", "proxy", "enabled"], true);
    set_nested_bool(root, &["networking", "proxy", "velocity", "enabled"], true);
    set_nested_string(root, &["networking", "proxy", "velocity", "secret"], secret);
    set_nested_string(root, &["networking", "java", "address"], bind);
    set_nested_bool(root, &["networking", "java", "online_mode"], false);
    set_nested_bool(root, &["networking", "java", "encryption"], false);
    // Velocity networks use Java only on backends; Bedrock defaults to UDP 19132 and breaks multi-instance.
    set_nested_bool(root, &["networking", "bedrock", "enabled"], false);
    set_nested_bool(root, &["networking", "lan_broadcast", "enabled"], false);
    let rcon_port = rcon_port_for_game_port(game_port);
    let rcon_password = rcon_password_for_server(secret, game_port);
    set_nested_bool(root, &["networking", "rcon", "enabled"], true);
    set_nested_string(
        root,
        &["networking", "rcon", "address"],
        &format!("127.0.0.1:{rcon_port}"),
    );
    set_nested_string(root, &["networking", "rcon", "password"], &rcon_password);
    // Legacy mistaken paths from earlier Pumpkin Patch builds
    remove_legacy_advanced_networking(root);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_patch_disables_bedrock() {
        let mut root = toml::from_str(
            r#"
[networking.java]
address = "0.0.0.0:25565"

[networking.bedrock]
enabled = true
"#,
        )
        .unwrap();
        apply_network_patch(&mut root, "127.0.0.1:25567", "secret", 25567);
        assert_eq!(
            root.get("networking")
                .and_then(|n| n.get("bedrock"))
                .and_then(|b| b.get("enabled"))
                .and_then(|v| v.as_bool()),
            Some(false)
        );
    }
}

fn remove_legacy_advanced_networking(root: &mut toml::Value) {
    if let Some(table) = root.as_table_mut() {
        table.remove("advanced");
    }
}

fn set_nested_string(root: &mut toml::Value, path: &[&str], value: &str) {
    set_nested(root, path, toml::Value::String(value.to_string()));
}

fn set_nested_bool(root: &mut toml::Value, path: &[&str], value: bool) {
    set_nested(root, path, toml::Value::Boolean(value));
}

fn set_nested(root: &mut toml::Value, path: &[&str], value: toml::Value) {
    if path.is_empty() {
        return;
    }
    let table = root
        .as_table_mut()
        .expect("config root must be a table");
    let mut current = table;
    for (i, key) in path.iter().enumerate() {
        if i == path.len() - 1 {
            current.insert(key.to_string(), value);
            return;
        }
        let entry = current
            .entry(key.to_string())
            .or_insert(toml::Value::Table(toml::map::Map::new()));
        if !entry.is_table() {
            *entry = toml::Value::Table(toml::map::Map::new());
        }
        current = entry.as_table_mut().expect("table");
    }
}
