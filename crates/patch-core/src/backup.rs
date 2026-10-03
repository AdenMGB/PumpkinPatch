use crate::db::Database;
use crate::error::{Error, Result};
use crate::models::{NetworkRecord, ServerRecord};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

const MANIFEST_NAME: &str = "patch-backup-manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkBackupManifest {
    pub version: u32,
    pub exported_at: String,
    pub network: NetworkRecord,
    pub servers: Vec<ServerRecord>,
}

pub async fn export_network_backup(
    db: &Database,
    network_id: Uuid,
    dest_zip: &Path,
) -> Result<()> {
    let network = db.get_network(network_id).await?;
    let servers = db.list_servers_for_network(network_id).await?;
    let manifest = NetworkBackupManifest {
        version: 1,
        exported_at: Utc::now().to_rfc3339(),
        network: network.clone(),
        servers,
    };

    let file = File::create(dest_zip)?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let manifest_raw = serde_json::to_string_pretty(&manifest)?;
    zip.start_file(MANIFEST_NAME, opts)
        .map_err(|e| Error::Other(format!("zip: {e}")))?;
    zip.write_all(manifest_raw.as_bytes())?;

    let data_root = PathBuf::from(&network.data_path);
    if data_root.exists() {
        add_dir_to_zip(&mut zip, &data_root, "data", opts)?;
    }

    zip.finish()
        .map_err(|e| Error::Other(format!("zip: {e}")))?;
    Ok(())
}

pub async fn import_network_backup(
    db: &Database,
    zip_path: &Path,
    networks_root: &Path,
    new_id: Option<Uuid>,
) -> Result<Uuid> {
    let file = File::open(zip_path)?;
    let mut archive =
        ZipArchive::new(file).map_err(|e| Error::Other(format!("zip: {e}")))?;
    let mut manifest_raw = String::new();
    {
        let mut manifest_file = archive
            .by_name(MANIFEST_NAME)
            .map_err(|e| Error::Other(format!("zip: {e}")))?;
        manifest_file.read_to_string(&mut manifest_raw)?;
    }
    let manifest: NetworkBackupManifest = serde_json::from_str(&manifest_raw)?;

    let network_id = new_id.unwrap_or_else(Uuid::new_v4);
    let new_root = networks_root.join(network_id.to_string());
    if new_root.exists() {
        return Err(Error::InvalidState(format!(
            "target network path already exists: {}",
            new_root.display()
        )));
    }
    std::fs::create_dir_all(&new_root)?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| Error::Other(format!("zip: {e}")))?;
        let name = file.name().to_string();
        if name == MANIFEST_NAME {
            continue;
        }
        let rel = name
            .strip_prefix("data/")
            .or_else(|| name.strip_prefix("data\\"))
            .unwrap_or(name.as_str());
        let out_path = new_root.join(rel);
        if name.ends_with('/') {
            std::fs::create_dir_all(&out_path)?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = File::create(&out_path)?;
        std::io::copy(&mut file, &mut out)?;
    }

    let old_data_path = manifest.network.data_path.clone();
    let mut network = manifest.network;
    network.id = network_id;
    network.data_path = new_root.to_string_lossy().into();
    network.created_at = Utc::now();

    db.insert_network(&network).await?;
    for mut server in manifest.servers {
        server.id = Uuid::new_v4();
        server.network_id = network_id;
        if let Ok(rel) = PathBuf::from(&server.data_path).strip_prefix(PathBuf::from(&old_data_path))
        {
            server.data_path = new_root.join(rel).to_string_lossy().into();
        } else {
            server.data_path = new_root.join("unknown").to_string_lossy().into();
        }
        server.created_at = Utc::now();
        db.insert_server(&server).await?;
    }

    Ok(network_id)
}

fn add_dir_to_zip(
    zip: &mut ZipWriter<File>,
    path: &Path,
    prefix: &str,
    opts: SimpleFileOptions,
) -> Result<()> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let p = entry.path();
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let zip_path = if prefix.is_empty() {
            file_name.clone()
        } else {
            format!("{prefix}/{file_name}")
        };
        if p.is_dir() {
            add_dir_to_zip(zip, &p, &zip_path, opts)?;
        } else {
            zip.start_file(&zip_path, opts)
                .map_err(|e| Error::Other(format!("zip: {e}")))?;
            let mut f = File::open(p)?;
            std::io::copy(&mut f, zip)?;
        }
    }
    Ok(())
}
