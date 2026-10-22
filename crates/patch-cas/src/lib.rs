//! Content-addressable blueprint blobs (scaffold for CoW provisioning).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasObject {
    pub digest: String,
    pub size: u64,
    pub path: PathBuf,
}

#[derive(Debug, Default)]
pub struct CasStore {
    root: PathBuf,
    index: HashMap<String, CasObject>,
}

impl CasStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        Self::open(&root).unwrap_or_else(|_| Self {
            root,
            index: HashMap::new(),
        })
    }

    /// Opens a store and indexes existing `sha256_*` blobs on disk.
    pub fn open(root: impl AsRef<Path>) -> std::io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root)?;
        let mut index = HashMap::new();
        for entry in std::fs::read_dir(&root)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("sha256_") {
                continue;
            }
            let digest = format!("sha256:{}", name.trim_start_matches("sha256_"));
            let meta = entry.metadata()?;
            let path = entry.path();
            let object = CasObject {
                digest: digest.clone(),
                size: meta.len(),
                path,
            };
            index.insert(digest, object);
        }
        Ok(Self { root, index })
    }

    pub fn digest_bytes(data: &[u8]) -> String {
        let hash = Sha256::digest(data);
        format!("sha256:{:x}", hash)
    }

    pub fn put_bytes(&mut self, data: &[u8]) -> std::io::Result<CasObject> {
        let digest = Self::digest_bytes(data);
        if let Some(existing) = self.index.get(&digest) {
            return Ok(existing.clone());
        }
        std::fs::create_dir_all(&self.root)?;
        let path = self.root.join(digest.replace(':', "_"));
        std::fs::write(&path, data)?;
        let object = CasObject {
            digest,
            size: data.len() as u64,
            path,
        };
        self.index.insert(object.digest.clone(), object.clone());
        Ok(object)
    }

    pub fn get(&self, digest: &str) -> Option<&CasObject> {
        self.index.get(digest)
    }

    /// Materializes a stored blueprint blob into `target_dir` (plangem CoW factory subset).
    pub fn materialize_blueprint(
        &self,
        digest: &str,
        target_dir: &Path,
    ) -> std::io::Result<PathBuf> {
        let source = if let Some(object) = self.get(digest) {
            object.path.clone()
        } else {
            let candidate = self.root.join(digest.replace(':', "_"));
            if candidate.is_file() {
                candidate
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "unknown digest",
                ));
            }
        };
        std::fs::create_dir_all(target_dir)?;
        let dest = target_dir.join("blueprint.bin");
        std::fs::copy(&source, &dest)?;
        Ok(dest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deduplicates_identical_payloads() {
        let mut store = CasStore::new(std::env::temp_dir().join("patch-cas-test"));
        let a = store.put_bytes(b"blueprint-v1").unwrap();
        let b = store.put_bytes(b"blueprint-v1").unwrap();
        assert_eq!(a.digest, b.digest);
    }

    #[test]
    fn open_indexes_existing_blob() {
        let root = std::env::temp_dir().join("patch-cas-open-test");
        let _ = std::fs::remove_dir_all(&root);
        let mut store = CasStore::new(&root);
        let object = store.put_bytes(b"persist-me").unwrap();
        let reopened = CasStore::open(&root).unwrap();
        assert!(reopened.get(&object.digest).is_some());
    }

    #[test]
    fn materialize_writes_blueprint_file() {
        let root = std::env::temp_dir().join("patch-cas-materialize-test");
        let mut store = CasStore::new(&root);
        let object = store.put_bytes(b"world-seed-42").unwrap();
        let out = std::env::temp_dir().join("patch-cas-materialize-out");
        let _ = std::fs::remove_dir_all(&out);
        let path = store.materialize_blueprint(&object.digest, &out).unwrap();
        assert!(path.is_file());
        assert_eq!(std::fs::read(path).unwrap(), b"world-seed-42");
    }
}
