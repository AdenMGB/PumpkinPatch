//! Country-level geolocation labels from client IPs (§2.4 subset — no raw IP storage).

use std::net::IpAddr;
use std::path::Path;

/// Environment variable pointing at a GeoLite2-Country `.mmdb` file (optional).
pub const GEOLITE_MMDB_ENV: &str = "PATCH_PLAN_GEOLITE_MMDB";

/// Resolves a display geolocation label from a connection IP string.
/// Private/loopback addresses map to `"Local Machine"`; unknown public IPs to `"Unknown"`
/// unless `PATCH_PLAN_GEOLITE_MMDB` is set and lookup succeeds.
pub fn geolocation_from_ip(ip: &str) -> String {
    let ip = ip.trim();
    if ip.is_empty() {
        return String::new();
    }
    let host = ip.split(':').next().unwrap_or(ip);
    match host.parse::<IpAddr>() {
        Ok(addr) if is_local_or_private(addr) => "Local Machine".to_string(),
        Ok(_) => geolite_country(host).unwrap_or_else(|| "Unknown".to_string()),
        Err(_) => "Unknown".to_string(),
    }
}

fn geolite_country(ip_host: &str) -> Option<String> {
    let path = std::env::var(GEOLITE_MMDB_ENV).ok()?;
    if path.is_empty() {
        return None;
    }
    lookup_geolite_country(Path::new(&path), ip_host)
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

fn is_local_or_private(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unique_local()
                || (v6.segments()[0] & 0xfe00) == 0xfc00 // unique local
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
        assert_eq!(geolocation_from_ip("8.8.8.8"), "Unknown");
    }
}
