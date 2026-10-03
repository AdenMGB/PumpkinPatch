use crate::error::Result;
use crate::java::FILL_USER_AGENT;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftVersionInfo {
    pub version: String,
    pub stable: bool,
}

pub async fn fetch_minecraft_versions() -> Result<Vec<MinecraftVersionInfo>> {
    let client = reqwest::Client::builder()
        .user_agent(FILL_USER_AGENT)
        .build()
        .map_err(|e| crate::error::Error::Other(e.to_string()))?;

    let mut merged: Vec<MinecraftVersionInfo> = Vec::new();
    let mut seen = HashSet::new();

    if let Ok(mojang) = fetch_mojang_versions(&client).await {
        for v in mojang {
            if seen.insert(v.version.clone()) {
                merged.push(v);
            }
        }
    }

    if let Ok(modrinth) = fetch_modrinth_versions(&client).await {
        for v in modrinth {
            if seen.insert(v.version.clone()) {
                merged.push(v);
            }
        }
    }

    if merged.is_empty() {
        merged = fallback_versions();
    }

    merged.sort_by(|a, b| compare_mc_version(&b.version, &a.version));
    Ok(merged)
}

async fn fetch_mojang_versions(client: &reqwest::Client) -> Result<Vec<MinecraftVersionInfo>> {
    let url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
    let manifest: MojangManifest = client.get(url).send().await?.json().await?;
    Ok(manifest
        .versions
        .into_iter()
        .map(|v| MinecraftVersionInfo {
            version: v.id,
            stable: v.version_type == "release",
        })
        .collect())
}

async fn fetch_modrinth_versions(client: &reqwest::Client) -> Result<Vec<MinecraftVersionInfo>> {
    let url = "https://api.modrinth.com/v2/tag/game_version";
    let tags: Vec<ModrinthGameVersion> = client
        .get(url)
        .header("User-Agent", FILL_USER_AGENT)
        .send()
        .await?
        .json()
        .await?;
    Ok(tags
        .into_iter()
        .filter(|t| t.version.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .map(|t| MinecraftVersionInfo {
            version: t.version,
            stable: t.version_type == "release",
        })
        .collect())
}

fn fallback_versions() -> Vec<MinecraftVersionInfo> {
    const DEFAULT: &[&str] = &[
        "1.21.11", "1.21.10", "1.21.9", "1.21.8", "1.21.7", "1.21.6", "1.21.5", "1.21.4",
        "1.21.3", "1.21.2", "1.21.1", "1.21", "1.20.6", "1.20.4", "1.20.2", "1.20.1", "1.20",
        "1.19.4", "1.18.2", "1.17.1", "1.16.5",
    ];
    DEFAULT
        .iter()
        .map(|v| MinecraftVersionInfo {
            version: (*v).into(),
            stable: true,
        })
        .collect()
}

fn compare_mc_version(a: &str, b: &str) -> std::cmp::Ordering {
    parse_mc_parts(a).cmp(&parse_mc_parts(b))
}

fn parse_mc_parts(id: &str) -> Vec<u32> {
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

#[derive(Debug, Deserialize)]
struct MojangManifest {
    versions: Vec<MojangVersionEntry>,
}

#[derive(Debug, Deserialize)]
struct MojangVersionEntry {
    id: String,
    #[serde(rename = "type")]
    version_type: String,
}

#[derive(Debug, Deserialize)]
struct ModrinthGameVersion {
    version: String,
    version_type: String,
}
