use crate::error::{Error, Result};
use crate::java::FILL_USER_AGENT;
use crate::server_settings::instance_plugins_dir;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

const MARKET_BASE: &str = "https://market.pumpkinmc.org";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpkinMarketListResult {
    pub items: Vec<PumpkinMarketPluginSummary>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpkinMarketPluginSummary {
    pub id: u64,
    pub name: String,
    pub version: String,
    pub category: String,
    pub dev_name: String,
    pub downloads: u64,
    pub preview_path: Option<String>,
    pub price_cents: u64,
    pub r#type: String,
    pub is_early_access: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpkinMarketPluginDetail {
    pub id: u64,
    pub name: String,
    pub version: String,
    pub category: String,
    pub dev_name: String,
    pub downloads: u64,
    pub preview_path: Option<String>,
    pub price_cents: u64,
    pub r#type: String,
    pub wasm_path: Option<String>,
    pub file_size: Option<u64>,
    pub description: String,
    pub owned: Option<bool>,
}

pub struct PumpkinMarketClient {
    http: reqwest::Client,
}

impl PumpkinMarketClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(FILL_USER_AGENT)
            .build()
            .expect("pumpkin market http client");
        Self { http }
    }

    pub async fn list_plugins(
        &self,
        search: Option<&str>,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<PumpkinMarketListResult> {
        let mut url = format!(
            "{}/api/plugins?limit={}&paginated=true",
            MARKET_BASE,
            limit.min(48)
        );
        if let Some(q) = search.filter(|s| !s.trim().is_empty()) {
            url.push_str(&format!("&search={}", urlencoding(q.trim())));
        }
        if let Some(c) = cursor.filter(|s| !s.is_empty()) {
            url.push_str(&format!("&cursor={}", urlencoding(c)));
        }
        let raw: MarketListResponse = self.get_json(&url).await?;
        Ok(PumpkinMarketListResult {
            items: raw
                .items
                .into_iter()
                .map(map_list_item)
                .collect(),
            has_more: raw.has_more,
            next_cursor: raw.next_cursor,
        })
    }

    pub async fn get_plugin(&self, plugin_id: u64) -> Result<PumpkinMarketPluginDetail> {
        let url = format!("{}/api/plugins/{}", MARKET_BASE, plugin_id);
        let raw: MarketPluginDetailRaw = self.get_json(&url).await?;
        Ok(map_detail(raw))
    }

    pub async fn install_plugin_to_instance(
        &self,
        plugin_id: u64,
        instance_path: &Path,
    ) -> Result<String> {
        let detail = self.get_plugin(plugin_id).await?;
        if detail.r#type != "free" && detail.price_cents > 0 && detail.owned != Some(true) {
            return Err(Error::Other(
                "This plugin requires purchase on market.pumpkinmc.org before download.".into(),
            ));
        }
        let filename = install_filename(&detail);
        let plugins_dir = instance_plugins_dir(instance_path);
        std::fs::create_dir_all(&plugins_dir)?;
        let dest = plugins_dir.join(&filename);
        let url = format!("{}/api/plugins/{}/download", MARKET_BASE, plugin_id);
        self.download_file(&url, &dest).await?;
        Ok(dest.to_string_lossy().into())
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let response = self.http.get(url).send().await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(Error::Other(format!("Pumpkin Market API {status}: {body}")));
        }
        serde_json::from_str(&body).map_err(|e| Error::Other(format!("Pumpkin Market JSON: {e}")))
    }

    async fn download_file(&self, url: &str, dest: &Path) -> Result<()> {
        let response = self.http.get(url).send().await?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Other(format!(
                "Pumpkin Market download failed ({status}): {body}"
            )));
        }
        let mut stream = response.bytes_stream();
        let mut file = tokio::fs::File::create(dest).await?;
        while let Some(chunk) = stream.next().await {
            file.write_all(&chunk?).await?;
        }
        file.flush().await?;
        Ok(())
    }
}

fn install_filename(detail: &PumpkinMarketPluginDetail) -> String {
    if let Some(path) = &detail.wasm_path {
        let name = PathBuf::from(path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned());
        if let Some(n) = name.filter(|n| n.ends_with(".wasm")) {
            return n;
        }
    }
    let slug: String = detail
        .name
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
        format!("plugin-{}.wasm", detail.id)
    } else {
        format!("{slug}.wasm")
    }
}

fn map_list_item(raw: MarketPluginSummaryRaw) -> PumpkinMarketPluginSummary {
    PumpkinMarketPluginSummary {
        id: raw.id,
        name: raw.name,
        version: raw.version,
        category: raw.category,
        dev_name: raw.dev_name,
        downloads: raw.downloads,
        preview_path: raw.preview_path,
        price_cents: raw.price_cents,
        r#type: raw.r#type,
        is_early_access: raw.is_early_access,
        description: pick_description(&raw.translated_descriptions),
    }
}

fn map_detail(raw: MarketPluginDetailRaw) -> PumpkinMarketPluginDetail {
    PumpkinMarketPluginDetail {
        id: raw.id,
        name: raw.name,
        version: raw.version,
        category: raw.category,
        dev_name: raw.dev_name,
        downloads: raw.downloads,
        preview_path: raw.preview_path,
        price_cents: raw.price_cents,
        r#type: raw.r#type,
        wasm_path: raw.wasm_path,
        file_size: raw.file_size,
        description: pick_description(&raw.translated_descriptions),
        owned: raw.owned,
    }
}

fn pick_description(translated: &HashMap<String, String>) -> String {
    translated
        .get("en-US")
        .or_else(|| translated.values().next())
        .cloned()
        .unwrap_or_default()
}

fn urlencoding(s: &str) -> String {
    urlencoding::encode(s).to_string()
}

#[derive(Debug, Deserialize)]
struct MarketListResponse {
    items: Vec<MarketPluginSummaryRaw>,
    has_more: bool,
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MarketPluginSummaryRaw {
    id: u64,
    name: String,
    version: String,
    category: String,
    dev_name: String,
    downloads: u64,
    preview_path: Option<String>,
    price_cents: u64,
    r#type: String,
    is_early_access: bool,
    translated_descriptions: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct MarketPluginDetailRaw {
    id: u64,
    name: String,
    version: String,
    category: String,
    dev_name: String,
    downloads: u64,
    preview_path: Option<String>,
    price_cents: u64,
    r#type: String,
    wasm_path: Option<String>,
    file_size: Option<u64>,
    translated_descriptions: HashMap<String, String>,
    owned: Option<bool>,
}
