use crate::error::{Error, Result};
use crate::java::FILL_USER_AGENT;
use crate::server_settings::instance_plugins_dir;
use futures_util::StreamExt;
use std::path::{Component, Path, PathBuf};
use tokio::io::AsyncWriteExt;

const WASM_MAGIC: &[u8; 4] = b"\0asm";

pub fn build_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(FILL_USER_AGENT)
        .build()
        .expect("patch-core http client")
}

pub fn is_plugin_artifact_filename(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".jar") || lower.ends_with(".wasm")
}

/// Reject absolute paths and `..` segments in manifest artifact paths.
pub fn resolve_bundled_artifact(plugins_root: &Path, artifact: &str) -> Result<PathBuf> {
    let rel = Path::new(artifact);
    if rel.is_absolute() {
        return Err(Error::InvalidState("plugin artifact must be relative".into()));
    }
    for component in rel.components() {
        if matches!(component, Component::ParentDir) {
            return Err(Error::InvalidState(
                "plugin artifact path must not contain ..".into(),
            ));
        }
    }
    Ok(plugins_root.join(rel))
}

pub fn discover_plugins_root(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates
        .iter()
        .find(|root| root.join("manifest.json").is_file())
        .cloned()
}

pub fn sanitize_plugin_filename(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidState("empty plugin filename".into()));
    }
    if trimmed.contains("..") || trimmed.contains('/') || trimmed.contains('\\') {
        return Err(Error::InvalidState("invalid plugin filename".into()));
    }
    if !is_plugin_artifact_filename(trimmed) {
        return Err(Error::InvalidState(
            "plugin filename must end with .wasm or .jar".into(),
        ));
    }
    Ok(trimmed.to_string())
}

pub fn slugify_plugin_name(name: &str, fallback_id: u64) -> String {
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        format!("plugin-{fallback_id}.wasm")
    } else {
        format!("{slug}.wasm")
    }
}

pub fn filename_from_wasm_path(wasm_path: &str) -> Option<String> {
    PathBuf::from(wasm_path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| n.to_ascii_lowercase().ends_with(".wasm"))
}

pub fn parse_content_disposition_filename(header: &str) -> Option<String> {
    for part in header.split(';').map(str::trim) {
        if let Some(rest) = part.strip_prefix("filename*=") {
            let value = rest
                .split_once("''")
                .map(|(_, v)| v)
                .unwrap_or(rest);
            if let Ok(decoded) = urlencoding::decode(value) {
                return sanitize_plugin_filename(&decoded).ok();
            }
        } else if let Some(raw) = part.strip_prefix("filename=") {
            let unquoted = raw.trim_matches('"').trim();
            return sanitize_plugin_filename(unquoted).ok();
        }
    }
    None
}

pub fn install_filename_from_market_detail(
    wasm_path: Option<&str>,
    name: &str,
    plugin_id: u64,
) -> String {
    if let Some(path) = wasm_path {
        if let Some(name) = filename_from_wasm_path(path) {
            return name;
        }
    }
    slugify_plugin_name(name, plugin_id)
}

pub fn copy_plugin_artifact(source: &Path, plugins_dir: &Path, filename: &str) -> Result<PathBuf> {
    std::fs::create_dir_all(plugins_dir)?;
    let safe_name = sanitize_plugin_filename(filename)?;
    let dest = plugins_dir.join(&safe_name);
    let tmp = dest.with_extension("part");
    if tmp.exists() {
        std::fs::remove_file(&tmp).ok();
    }
    std::fs::copy(source, &tmp)?;
    std::fs::rename(tmp, &dest)?;
    Ok(dest)
}

pub async fn download_plugin_to_instance(
    client: &reqwest::Client,
    url: &str,
    instance_path: &Path,
    preferred_filename: &str,
    validate_wasm: bool,
) -> Result<PathBuf> {
    let plugins_dir = instance_plugins_dir(instance_path);
    std::fs::create_dir_all(&plugins_dir)?;

    let response = client.get(url).send().await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Other(format!("plugin download failed ({status}): {body}")));
    }

    let header_name = response
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_content_disposition_filename);

    let filename = header_name
        .or_else(|| sanitize_plugin_filename(preferred_filename).ok())
        .ok_or_else(|| Error::InvalidState("could not determine plugin filename".into()))?;

    let dest = plugins_dir.join(&filename);
    let tmp = dest.with_extension("part");
    if tmp.exists() {
        tokio::fs::remove_file(&tmp).await.ok();
    }

    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(&tmp).await?;
    while let Some(chunk) = stream.next().await {
        file.write_all(&chunk?).await?;
    }
    file.flush().await?;
    drop(file);

    if validate_wasm {
        validate_wasm_file(&tmp)?;
    }

    if dest.exists() {
        tokio::fs::remove_file(&dest).await.ok();
    }
    tokio::fs::rename(&tmp, &dest).await?;
    Ok(dest)
}

fn validate_wasm_file(path: &Path) -> Result<()> {
    let meta = std::fs::metadata(path)?;
    if meta.len() < 8 {
        return Err(Error::InvalidState("downloaded plugin file is too small".into()));
    }
    let mut head = [0u8; 4];
    let mut f = std::fs::File::open(path)?;
    use std::io::Read;
    f.read_exact(&mut head)?;
    if &head != WASM_MAGIC {
        return Err(Error::InvalidState(
            "downloaded file is not a valid WASM plugin".into(),
        ));
    }
    Ok(())
}

pub fn cache_bundled_plugin(
    cache_dir: &Path,
    plugin_id: &str,
    version: &str,
    source: &Path,
) -> Result<()> {
    std::fs::create_dir_all(cache_dir)?;
    let cached = cache_dir.join(format!("{plugin_id}-{version}.wasm"));
    std::fs::copy(source, cached)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn rejects_parent_dir_in_artifact() {
        let root = PathBuf::from("/tmp/plugins");
        assert!(resolve_bundled_artifact(&root, "../evil.wasm").is_err());
    }

    #[test]
    fn parses_content_disposition() {
        let name = parse_content_disposition_filename(
            "attachment; filename=\"VoidWorld.wasm\"; filename*=UTF-8''VoidWorld.wasm",
        );
        assert_eq!(name.as_deref(), Some("VoidWorld.wasm"));
    }

    #[test]
    fn slugify_handles_empty_name() {
        assert_eq!(slugify_plugin_name("!!!", 42), "plugin-42.wasm");
    }

    #[test]
    fn copy_is_atomic() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.wasm");
        let mut f = std::fs::File::create(&source).unwrap();
        f.write_all(b"\0asm\x01\0\0\0").unwrap();
        let plugins = dir.path().join("plugins");
        let dest = copy_plugin_artifact(&source, &plugins, "test.wasm").unwrap();
        assert!(dest.exists());
        assert!(!dest.with_extension("part").exists());
    }
}
