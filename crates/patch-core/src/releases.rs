use crate::error::{Error, Result};
use crate::java::FILL_USER_AGENT;
use crate::state::AppDirs;
use futures_util::StreamExt;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostOs {
    Windows,
    Linux,
    MacOs,
}

impl HostOs {
    fn as_slug(self) -> &'static str {
        match self {
            Self::Windows => "Windows",
            Self::Linux => "Linux",
            Self::MacOs => "MacOS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostArch {
    X64,
    Arm64,
}

pub fn detect_host() -> Result<(HostOs, HostArch)> {
    match std::env::consts::OS {
        "windows" => {
            let arch = match std::env::consts::ARCH {
                "x86_64" => HostArch::X64,
                "aarch64" => HostArch::Arm64,
                _ => return Err(Error::UnsupportedPlatform(std::env::consts::ARCH.into())),
            };
            Ok((HostOs::Windows, arch))
        }
        "linux" => {
            let arch = match std::env::consts::ARCH {
                "x86_64" => HostArch::X64,
                "aarch64" => HostArch::Arm64,
                _ => return Err(Error::UnsupportedPlatform(std::env::consts::ARCH.into())),
            };
            Ok((HostOs::Linux, arch))
        }
        "macos" => {
            let arch = match std::env::consts::ARCH {
                "x86_64" => HostArch::X64,
                "aarch64" => HostArch::Arm64,
                _ => return Err(Error::UnsupportedPlatform(std::env::consts::ARCH.into())),
            };
            Ok((HostOs::MacOs, arch))
        }
        other => Err(Error::UnsupportedPlatform(other.into())),
    }
}

pub struct PumpkinArtifact {
    pub url: String,
    pub filename: String,
}

pub fn resolve_pumpkin_artifact(
    os: HostOs,
    arch: HostArch,
    channel: &str,
) -> Result<PumpkinArtifact> {
    let arch_slug = match arch {
        HostArch::X64 => "X64",
        HostArch::Arm64 => "ARM64",
    };
    let filename = match os {
        HostOs::Windows => format!("pumpkin-{arch_slug}-Windows.exe"),
        HostOs::Linux => format!("pumpkin-{arch_slug}-Linux"),
        HostOs::MacOs => format!("pumpkin-{arch_slug}-MacOS"),
    };
    let tag = match channel {
        "latest" | "nightly" => "nightly",
        other => other,
    };
    let url = format!(
        "https://github.com/Pumpkin-MC/Pumpkin/releases/download/{tag}/{filename}"
    );
    Ok(PumpkinArtifact {
        url,
        filename,
    })
}

#[derive(Clone)]
pub struct ReleaseService {
    dirs: AppDirs,
    client: reqwest::Client,
}

impl ReleaseService {
    pub fn new(dirs: AppDirs) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(FILL_USER_AGENT)
            .build()
            .expect("http client");
        Self { dirs, client }
    }

    pub fn pumpkin_binary_path(&self, channel: &str, filename: &str) -> PathBuf {
        self.dirs
            .binaries_pumpkin
            .join(channel)
            .join(filename)
    }

    pub fn velocity_jar_path(&self, filename: &str) -> PathBuf {
        self.dirs.binaries_velocity.join(filename)
    }

    pub async fn ensure_pumpkin_binary(
        &self,
        channel: &str,
        progress: Option<mpsc::Sender<DownloadProgress>>,
    ) -> Result<PathBuf> {
        let (os, arch) = detect_host()?;
        let artifact = resolve_pumpkin_artifact(os, arch, channel)?;
        let dest = self.pumpkin_binary_path(channel, &artifact.filename);
        if dest.exists() {
            return Ok(dest);
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.download_file(&artifact.url, &dest, &artifact.filename, progress)
            .await?;
        Ok(dest)
    }

    pub async fn ensure_velocity_jar(
        &self,
        progress: Option<mpsc::Sender<DownloadProgress>>,
    ) -> Result<PathBuf> {
        let (filename, url) = self.fetch_recommended_velocity_download().await?;
        let dest = self.velocity_jar_path(&filename);
        if dest.exists() {
            return Ok(dest);
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.download_file(&url, &dest, &filename, progress).await?;
        Ok(dest)
    }

    async fn fetch_recommended_velocity_download(&self) -> Result<(String, String)> {
        let project_url = "https://fill.papermc.io/v3/projects/velocity";
        let project: FillProjectResponse = self.get_json(project_url).await?;

        let versions = flatten_fill_versions(&project.versions);
        for version in versions {
            if version.contains("SNAPSHOT") {
                continue;
            }
            let builds_url = format!(
                "https://fill.papermc.io/v3/projects/velocity/versions/{version}/builds"
            );
            let builds: Vec<FillBuild> = match self.get_json(&builds_url).await {
                Ok(b) => b,
                Err(_) => continue,
            };
            if let Some(build) = select_velocity_build(&builds) {
                let download = build
                    .downloads
                    .get("server:default")
                    .ok_or_else(|| Error::Other("velocity build missing server:default".into()))?;
                return Ok((download.name.clone(), download.url.clone()));
            }
        }

        Err(Error::Other(
            "could not find a Velocity build from PaperMC Fill API".into(),
        ))
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let response = self.client.get(url).send().await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(Error::Other(format!(
                "PaperMC Fill API error {status} for {url}: {body}"
            )));
        }
        serde_json::from_str(&body).map_err(|e| {
            Error::Other(format!(
                "PaperMC Fill API returned unexpected JSON for {url}: {e}; body: {body}"
            ))
        })
    }

    async fn download_file(
        &self,
        url: &str,
        dest: &Path,
        label: &str,
        progress: Option<mpsc::Sender<DownloadProgress>>,
    ) -> Result<()> {
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(Error::Other(format!(
                "download failed {}: {}",
                url,
                response.status()
            )));
        }
        let total = response.content_length();
        let mut stream = response.bytes_stream();
        let mut file = tokio::fs::File::create(dest).await?;
        let mut downloaded: u64 = 0;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            if let Some(tx) = &progress {
                let _ = tx.try_send(DownloadProgress {
                    downloaded,
                    total,
                    label: label.to_string(),
                });
            }
        }
        file.flush().await?;
        Ok(())
    }
}

fn flatten_fill_versions(groups: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    let mut all: Vec<String> = groups.values().flatten().cloned().collect();
    all.sort_by(|a, b| compare_version_ids(b, a));
    all
}

fn compare_version_ids(a: &str, b: &str) -> std::cmp::Ordering {
    let pa = parse_version_parts(a);
    let pb = parse_version_parts(b);
    pa.cmp(&pb)
}

fn parse_version_parts(id: &str) -> Vec<u32> {
    id.split(|c| c == '.' || c == '-')
        .filter_map(|p| {
            if p.eq_ignore_ascii_case("SNAPSHOT") {
                None
            } else {
                p.parse().ok()
            }
        })
        .collect()
}

fn select_velocity_build(builds: &[FillBuild]) -> Option<&FillBuild> {
    builds
        .iter()
        .rev()
        .find(|b| {
            (b.channel == "RECOMMENDED" || b.channel == "STABLE")
                && b.downloads.contains_key("server:default")
        })
}

#[derive(Debug, Deserialize)]
struct FillProjectResponse {
    versions: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct FillBuild {
    channel: String,
    downloads: BTreeMap<String, FillDownload>,
}

#[derive(Debug, Deserialize)]
struct FillDownload {
    name: String,
    url: String,
}

#[cfg(test)]
mod tests {
    use super::{resolve_pumpkin_artifact, HostArch, HostOs};

    #[test]
    fn windows_nightly_url() {
        let artifact = resolve_pumpkin_artifact(HostOs::Windows, HostArch::X64, "nightly").unwrap();
        assert!(artifact.url.contains("nightly"));
        assert!(artifact.filename.contains("Windows"));
    }
}
