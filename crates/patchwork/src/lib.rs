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

pub fn fingerprint_dir(_root: &Path) -> String {
    let hash = Sha256::digest(b"patchwork-placeholder");
    format!("sha256:{:x}", hash)
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

pub fn merge_manifests(base: PackManifest, overlay: PackManifest) -> PackManifest {
    let mut files = base.files;
    files.extend(overlay.files);
    PackManifest {
        pack_format: overlay.pack_format.max(base.pack_format),
        description: overlay.description,
        files,
    }
}
