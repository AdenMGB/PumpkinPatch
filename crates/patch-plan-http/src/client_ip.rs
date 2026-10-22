//! Client IP extraction for Plan access logs (indepth.md §5.2).

pub fn client_ip_from_headers(
    headers: &axum::http::HeaderMap,
    trust_forwarded: bool,
) -> Option<String> {
    if trust_forwarded {
        if let Some(value) = headers.get("x-forwarded-for") {
            if let Ok(raw) = value.to_str() {
                let ip = raw.split(',').next().unwrap_or(raw).trim();
                if !ip.is_empty() {
                    return Some(ip.to_string());
                }
            }
        }
        if let Some(value) = headers.get("x-real-ip") {
            if let Ok(ip) = value.to_str() {
                if !ip.is_empty() {
                    return Some(ip.to_string());
                }
            }
        }
    }
    None
}
