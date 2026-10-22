//! GeoLite2-Country cache for Plan geolocation (indepth.md §2.4).

use crate::error::{Error, Result};
use patch_plan_core::set_geolite_mmdb_path;
use std::path::{Path, PathBuf};

pub const GEOLITE_DOWNLOAD_URL: &str =
    "https://geodb.playeranalytics.net/GeoLite2-Country.mmdb";

pub fn geolite_cache_path(network_data_path: &Path) -> PathBuf {
    network_data_path.join("plan/geo/GeoLite2-Country.mmdb")
}

/// Ensures the MMDB exists when EULA is accepted, then registers it for lookups.
pub async fn configure_network_geolocation(
    network_data_path: &Path,
    accept_geolite_eula: bool,
) -> Result<()> {
    let cache_path = geolite_cache_path(network_data_path);
    if accept_geolite_eula {
        ensure_geolite_mmdb(&cache_path).await?;
        set_geolite_mmdb_path(cache_path);
    } else if cache_path.is_file() {
        set_geolite_mmdb_path(cache_path);
    }
    Ok(())
}

async fn ensure_geolite_mmdb(path: &Path) -> Result<()> {
    if path.is_file() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| Error::Other(format!("geolite http client: {e}")))?;
    let bytes = client
        .get(GEOLITE_DOWNLOAD_URL)
        .send()
        .await
        .map_err(|e| Error::Other(format!("geolite download: {e}")))?
        .error_for_status()
        .map_err(|e| Error::Other(format!("geolite download status: {e}")))?
        .bytes()
        .await
        .map_err(|e| Error::Other(format!("geolite body: {e}")))?;
    if bytes.len() < 1024 {
        return Err(Error::InvalidState(
            "geolite download returned unexpectedly small payload".into(),
        ));
    }
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_path_under_plan_geo() {
        let p = geolite_cache_path(Path::new("/data/net"));
        assert!(p.ends_with("plan/geo/GeoLite2-Country.mmdb"));
    }
}
