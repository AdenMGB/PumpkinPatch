//! PatchWork resource-pack merge helpers (scaffold).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackManifest {
    pub pack_format: u32,
    pub description: String,
    pub files: BTreeMap<String, String>,
}

pub fn fingerprint_dir(root: &Path) -> String {
    match index_pack_assets(root) {
        Ok(files) => {
            let mut payload = String::new();
            for (path, digest) in files {
                payload.push_str(&path);
                payload.push(':');
                payload.push_str(&digest);
                payload.push('\n');
            }
            let hash = Sha256::digest(payload.as_bytes());
            format!("sha256:{:x}", hash)
        }
        Err(_) => {
            let hash = Sha256::digest(b"patchwork-empty");
            format!("sha256:{:x}", hash)
        }
    }
}

/// Read `pack.mcmeta` from a resource-pack root (plangem.md PatchWork phase-4 scaffold).
pub fn read_pack_manifest(root: &Path) -> Result<PackManifest, String> {
    let path = root.join("pack.mcmeta");
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let pack = v.get("pack").and_then(|p| p.as_object());
    let pack_format = pack
        .and_then(|p| p.get("pack_format"))
        .and_then(|n| n.as_u64())
        .unwrap_or(34) as u32;
    let description = pack
        .and_then(|p| p.get("description"))
        .and_then(|d| d.as_str())
        .unwrap_or("PatchWork pack")
        .to_string();
    Ok(PackManifest {
        pack_format,
        description,
        files: BTreeMap::new(),
    })
}

/// Walk a pack directory and record BLAKE3-style sha256 paths (PatchWork asset index).
pub fn index_pack_assets(root: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut files = BTreeMap::new();
    walk_pack(root, root, &mut files)?;
    Ok(files)
}

fn walk_pack(
    root: &Path,
    dir: &Path,
    out: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            walk_pack(root, &path, out)?;
        } else if path.file_name().and_then(|n| n.to_str()) != Some("pack.mcmeta") {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            let hash = Sha256::digest(&bytes);
            out.insert(rel, format!("sha256:{:x}", hash));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn materialize_overlay_wins() {
        let base = std::env::temp_dir().join("patchwork-base");
        let overlay = std::env::temp_dir().join("patchwork-overlay");
        let out = std::env::temp_dir().join("patchwork-out");
        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::remove_dir_all(&overlay);
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(base.join("assets")).unwrap();
        std::fs::write(base.join("pack.mcmeta"), r#"{"pack":{"pack_format":34,"description":"b"}}"#)
            .unwrap();
        std::fs::write(base.join("assets/a.txt"), b"base").unwrap();
        std::fs::create_dir_all(overlay.join("assets")).unwrap();
        std::fs::write(
            overlay.join("pack.mcmeta"),
            r#"{"pack":{"pack_format":34,"description":"o"}}"#,
        )
        .unwrap();
        std::fs::write(overlay.join("assets/a.txt"), b"overlay").unwrap();
        let manifest = materialize_merged_pack(&base, &overlay, &out).unwrap();
        assert_eq!(manifest.description, "o");
        let body = std::fs::read_to_string(out.join("assets/a.txt")).unwrap();
        assert_eq!(body, "overlay");
    }

    #[test]
    fn indexes_pack_files() {
        let dir = std::env::temp_dir().join("patchwork-index-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("assets/minecraft/textures")).unwrap();
        let mut f = std::fs::File::create(dir.join("assets/minecraft/textures/a.png")).unwrap();
        f.write_all(b"png").unwrap();
        std::fs::write(
            dir.join("pack.mcmeta"),
            r#"{"pack":{"pack_format":34,"description":"t"}}"#,
        )
        .unwrap();
        let map = index_pack_assets(&dir).unwrap();
        assert!(map.contains_key("assets/minecraft/textures/a.png"));
    }
}

pub fn merge_manifests(base: PackManifest, overlay: PackManifest) -> PackManifest {
    let mut files = base.files;
    files.extend(overlay.files);
    PackManifest {
        pack_format: overlay.pack_format.max(base.pack_format),
        description: overlay.description,
        files,
    }
}

fn copy_pack_tree(from: &Path, to: &Path) -> Result<(), String> {
    for entry in std::fs::read_dir(from).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let dest = to.join(entry.file_name());
        if path.is_dir() {
            std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
            copy_pack_tree(&path, &dest)?;
        } else {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::copy(&path, &dest).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Copy base pack then overlay (wins on conflicts) into `out` (plangem.md PatchWork flows).
pub fn materialize_merged_pack(
    base: &Path,
    overlay: &Path,
    out: &Path,
) -> Result<PackManifest, String> {
    if out.exists() {
        std::fs::remove_dir_all(out).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    copy_pack_tree(base, out)?;
    copy_pack_tree(overlay, out)?;
    let mut merged = read_pack_manifest(out)?;
    merged.files = index_pack_assets(out)?;
    Ok(merged)
}
