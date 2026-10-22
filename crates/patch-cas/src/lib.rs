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
        Self {
            root: root.as_ref().to_path_buf(),
            index: HashMap::new(),
        }
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
}
