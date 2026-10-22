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

pub fn merge_manifests(base: PackManifest, overlay: PackManifest) -> PackManifest {
    let mut files = base.files;
    files.extend(overlay.files);
    PackManifest {
        pack_format: overlay.pack_format.max(base.pack_format),
        description: overlay.description,
        files,
    }
}
