use crate::error::{Error, Result};
use crate::plugin_install::{
    build_http_client, download_plugin_to_instance, install_filename_from_market_detail,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

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

impl Default for PumpkinMarketClient {
    fn default() -> Self {
        Self::new()
    }
}

impl PumpkinMarketClient {
    pub fn new() -> Self {
        Self {
            http: build_http_client(),
        }
    }

    pub async fn list_plugins(
        &self,
        search: Option<&str>,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<PumpkinMarketListResult> {
        let limit = limit.clamp(1, 48);
        let mut url = reqwest::Url::parse(&format!("{MARKET_BASE}/api/plugins"))
            .map_err(|e| Error::Other(e.to_string()))?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("limit", &limit.to_string());
            pairs.append_pair("paginated", "true");
            if let Some(q) = search.filter(|s| !s.trim().is_empty()) {
                pairs.append_pair("search", q.trim());
            }
            if let Some(c) = cursor.filter(|s| !s.is_empty()) {
                pairs.append_pair("cursor", c);
            }
        }
        let raw: MarketListResponse = self.get_json(url.as_str()).await?;
        Ok(PumpkinMarketListResult {
            items: raw.items.into_iter().map(map_list_item).collect(),
            has_more: raw.has_more,
            next_cursor: raw.next_cursor,
        })
    }

    pub async fn get_plugin(&self, plugin_id: u64) -> Result<PumpkinMarketPluginDetail> {
        let url = format!("{MARKET_BASE}/api/plugins/{plugin_id}");
        let raw: MarketPluginDetailRaw = self.get_json(&url).await?;
        Ok(map_detail(raw))
    }

    pub async fn install_plugin_to_instance(
        &self,
        plugin_id: u64,
        instance_path: &Path,
    ) -> Result<String> {
        let detail = self.get_plugin(plugin_id).await?;
        ensure_plugin_downloadable(&detail)?;

        let filename = install_filename_from_market_detail(
            detail.wasm_path.as_deref(),
            &detail.name,
            detail.id,
        );
        let url = format!("{MARKET_BASE}/api/plugins/{plugin_id}/download");
        let dest = download_plugin_to_instance(&self.http, &url, instance_path, &filename, true)
            .await?;

        if let Some(expected) = detail.file_size {
            let actual = std::fs::metadata(&dest)?.len();
            if expected > 0 && actual != expected {
                std::fs::remove_file(&dest).ok();
                return Err(Error::InvalidState(format!(
                    "download size mismatch for {} (expected {expected}, got {actual})",
                    detail.name
                )));
            }
        }

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
}

fn ensure_plugin_downloadable(detail: &PumpkinMarketPluginDetail) -> Result<()> {
    let is_free = detail.r#type.eq_ignore_ascii_case("free") || detail.price_cents == 0;
    if is_free {
        return Ok(());
    }
    if detail.owned == Some(true) {
        return Ok(());
    }
    Err(Error::Other(format!(
        "Purchase \"{}\" on market.pumpkinmc.org before installing in Pumpkin Patch.",
        detail.name
    )))
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
        .or_else(|| translated.get("en"))
        .or_else(|| translated.values().next())
        .cloned()
        .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_plugins_always_downloadable() {
        let detail = PumpkinMarketPluginDetail {
            id: 1,
            name: "Test".into(),
            version: "1".into(),
            category: "Other".into(),
            dev_name: "dev".into(),
            downloads: 0,
            preview_path: None,
            price_cents: 0,
            r#type: "free".into(),
            wasm_path: None,
            file_size: None,
            description: String::new(),
            owned: None,
        };
        assert!(ensure_plugin_downloadable(&detail).is_ok());
    }

    #[test]
    fn paid_requires_ownership() {
        let detail = PumpkinMarketPluginDetail {
            id: 1,
            name: "Paid".into(),
            version: "1".into(),
            category: "Other".into(),
            dev_name: "dev".into(),
            downloads: 0,
            preview_path: None,
            price_cents: 500,
            r#type: "paid".into(),
            wasm_path: None,
            file_size: None,
            description: String::new(),
            owned: None,
        };
        assert!(ensure_plugin_downloadable(&detail).is_err());
    }
}
