//! Country-level geolocation labels from client IPs (indepth.md §2.4).

use std::collections::HashMap;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, RwLock};

/// Environment variable pointing at a GeoLite2-Country `.mmdb` file (optional).
pub const GEOLITE_MMDB_ENV: &str = "PATCH_PLAN_GEOLITE_MMDB";

static CONFIGURED_MMDB: OnceLock<PathBuf> = OnceLock::new();
static IP_API_CACHE: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();

/// Registers a downloaded GeoLite2 database for subsequent lookups on this process.
pub fn set_geolite_mmdb_path(path: PathBuf) {
    let _ = CONFIGURED_MMDB.set(path);
}

/// Resolves a display geolocation label from a connection IP string.
pub fn geolocation_from_ip(ip: &str) -> String {
    let ip = ip.trim();
    if ip.is_empty() {
        return String::new();
    }
    let host = ip.split(':').next().unwrap_or(ip);
    match host.parse::<IpAddr>() {
        Ok(addr) if is_local_or_private(addr) => "Local Machine".to_string(),
        Ok(_) => geolite_country(host)
            .or_else(|| ip_api_country(host))
            .unwrap_or_else(|| "Unknown".to_string()),
        Err(_) => "Unknown".to_string(),
    }
}

fn geolite_country(ip_host: &str) -> Option<String> {
    if let Ok(path) = std::env::var(GEOLITE_MMDB_ENV) {
        if !path.is_empty() {
            if let Some(c) = lookup_geolite_country(Path::new(&path), ip_host) {
                return Some(c);
            }
        }
    }
    if let Some(path) = CONFIGURED_MMDB.get() {
        return lookup_geolite_country(path, ip_host);
    }
    None
}

fn lookup_geolite_country(mmdb_path: &Path, ip_host: &str) -> Option<String> {
    let ip: IpAddr = ip_host.parse().ok()?;
    let reader = maxminddb::Reader::open_readfile(mmdb_path).ok()?;
    let country: maxminddb::geoip2::Country = reader.lookup(ip).ok()?;
    country
        .country?
        .names?
        .get("en")
        .map(|s| s.to_string())
}

fn ip_api_cache() -> &'static RwLock<HashMap<String, String>> {
    IP_API_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// HTTP fallback when GeoLite2 is unavailable (indepth §2.4 step 3).
fn ip_api_country(ip_host: &str) -> Option<String> {
    if std::env::var("PATCH_PLAN_DISABLE_IP_API").is_ok() {
        return None;
    }
    {
        let cache = ip_api_cache().read().expect("ip-api cache lock");
        if let Some(c) = cache.get(ip_host) {
            return Some(c.clone());
        }
    }
    static IP_API_LOCK: Mutex<()> = Mutex::new(());
    let _guard = IP_API_LOCK.lock().expect("ip-api lock");
    {
        let cache = ip_api_cache().read().expect("ip-api cache lock");
        if let Some(c) = cache.get(ip_host) {
            return Some(c.clone());
        }
    }
    let url = format!(
        "http://ip-api.com/json/{}?fields=status,country,countryCode",
        ip_host
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
        .ok()?;
    let body: serde_json::Value = client.get(url).send().ok()?.json().ok()?;
    if body.get("status").and_then(|s| s.as_str()) != Some("success") {
        return None;
    }
    let country = body
        .get("country")
        .and_then(|c| c.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())?;
    if ip_api_cache().read().expect("ip-api cache lock").len() < 50_000 {
        ip_api_cache()
            .write()
            .expect("ip-api cache lock")
            .insert(ip_host.to_string(), country.clone());
    }
    Some(country)
}

fn is_local_or_private(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unique_local()
                || (v6.segments()[0] & 0xfe00) == 0xfc00
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_is_local_machine() {
        assert_eq!(geolocation_from_ip("127.0.0.1"), "Local Machine");
    }

    #[test]
    fn private_lan_is_local_machine() {
        assert_eq!(geolocation_from_ip("192.168.1.42"), "Local Machine");
    }

    #[test]
    fn public_ip_is_unknown_without_geo_db() {
        std::env::set_var("PATCH_PLAN_DISABLE_IP_API", "1");
        assert_eq!(geolocation_from_ip("8.8.8.8"), "Unknown");
        std::env::remove_var("PATCH_PLAN_DISABLE_IP_API");
    }
}
