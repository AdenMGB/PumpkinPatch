use crate::error::{Error, Result};
use crate::plugin_install::{build_http_client, download_plugin_to_instance};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthSearchHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthSearchResult {
    pub hits: Vec<ModrinthSearchHit>,
    pub total_hits: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthVersion {
    pub id: String,
    pub version_number: String,
    pub name: String,
    pub game_versions: Vec<String>,
    pub date_published: String,
    pub files: Vec<ModrinthVersionFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthVersionFile {
    pub filename: String,
    pub url: String,
    pub primary: bool,
}

pub struct ModrinthClient {
    http: reqwest::Client,
}

impl Default for ModrinthClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ModrinthClient {
    pub fn new() -> Self {
        Self {
            http: build_http_client(),
        }
    }

    pub async fn search_plugins(
        &self,
        query: &str,
        minecraft_version: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<ModrinthSearchResult> {
        let mut facets: Vec<Vec<String>> = vec![vec!["project_type:plugin".into()]];
        if let Some(v) = minecraft_version {
            facets.push(vec![format!("versions:{v}")]);
        }
        let facets_json = serde_json::to_string(&facets)?;
        let url = format!(
            "https://api.modrinth.com/v2/search?query={}&facets={}&limit={}&offset={}&index=relevance",
            urlencoding(query),
            urlencoding(&facets_json),
            limit,
            offset
        );
        let resp: ModrinthSearchResponse = self.get_json(&url).await?;
        Ok(ModrinthSearchResult {
            hits: resp
                .hits
                .into_iter()
                .map(|h| ModrinthSearchHit {
                    project_id: h.project_id,
                    slug: h.slug,
                    title: h.title,
                    description: h.description,
                    icon_url: h.icon_url,
                    downloads: h.downloads,
                    author: h.author,
                })
                .collect(),
            total_hits: resp.total_hits,
        })
    }

    pub async fn get_version(&self, version_id: &str) -> Result<ModrinthVersion> {
        let url = format!("https://api.modrinth.com/v2/version/{}", urlencoding(version_id));
        self.get_json(&url).await
    }

    pub async fn project_versions(
        &self,
        project_id: &str,
        minecraft_version: Option<&str>,
    ) -> Result<Vec<ModrinthVersion>> {
        let mut url = format!(
            "https://api.modrinth.com/v2/project/{}/version",
            urlencoding(project_id)
        );
        if let Some(v) = minecraft_version {
            url.push_str(&format!("?game_versions=[\"{}\"]", v));
        }
        self.get_json(&url).await
    }

    pub async fn install_version_to_instance(
        &self,
        version: &ModrinthVersion,
        instance_path: &Path,
    ) -> Result<String> {
        let file = version
            .files
            .iter()
            .find(|f| f.primary)
            .or_else(|| version.files.first())
            .ok_or_else(|| Error::Other("version has no downloadable files".into()))?;
        let validate_wasm = file.filename.to_ascii_lowercase().ends_with(".wasm");
        let dest = download_plugin_to_instance(
            &self.http,
            &file.url,
            instance_path,
            &file.filename,
            validate_wasm,
        )
        .await?;
        Ok(dest.to_string_lossy().into())
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let response = self.http.get(url).send().await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(Error::Other(format!("Modrinth API {status}: {body}")));
        }
        serde_json::from_str(&body).map_err(|e| Error::Other(format!("Modrinth JSON: {e}")))
    }
}

fn urlencoding(s: &str) -> String {
    urlencoding::encode(s).to_string()
}

#[derive(Debug, Deserialize)]
struct ModrinthSearchResponse {
    hits: Vec<ModrinthSearchHitRaw>,
    total_hits: u32,
}

#[derive(Debug, Deserialize)]
struct ModrinthSearchHitRaw {
    project_id: String,
    slug: String,
    title: String,
    description: String,
    icon_url: Option<String>,
    downloads: u64,
    author: String,
}
