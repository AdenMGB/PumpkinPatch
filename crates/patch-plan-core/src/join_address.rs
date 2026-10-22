//! Join address normalization — Plan §2.1

#[derive(Debug, Clone, Default)]
pub struct JoinAddressOptions {
    pub preserve_case: bool,
    pub preserve_invalid: bool,
    pub filter_out: Vec<String>,
}

pub fn normalize_join_address(raw: &str, opts: &JoinAddressOptions) -> String {
    let mut host = raw.split('\0').next().unwrap_or(raw).trim();
    if let Some((base, _)) = host.rsplit_once(':') {
        if base.chars().all(|c| c.is_ascii_digit() || c == '.') || host.contains(':') {
            host = base;
        }
    }
    host = host.trim_end_matches('.');
    let out = if opts.preserve_case {
        host.to_string()
    } else {
        host.to_ascii_lowercase()
    };
    if out.is_empty() {
        return "unknown".into();
    }
    if !opts.preserve_invalid && !is_valid_host(&out) {
        return "unknown".into();
    }
    let filtered = opts
        .filter_out
        .iter()
        .any(|f| out.eq_ignore_ascii_case(f));
    if filtered {
        return "Unknown".into();
    }
    out
}

fn is_valid_host(host: &str) -> bool {
    if host == "localhost" {
        return true;
    }
    if looks_like_ipv4(host) || looks_like_ipv6(host) {
        return true;
    }
    if host.len() > 253 {
        return false;
    }
    host.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    })
}

fn looks_like_ipv4(host: &str) -> bool {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    parts.iter().all(|p| p.parse::<u8>().is_ok())
}

fn looks_like_ipv6(host: &str) -> bool {
    host.contains(':')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_bungee_suffix() {
        let opts = JoinAddressOptions::default();
        assert_eq!(
            normalize_join_address("mc.example.com\0FML\0", &opts),
            "mc.example.com"
        );
    }

    #[test]
    fn lowercases_by_default() {
        let opts = JoinAddressOptions::default();
        assert_eq!(normalize_join_address("MC.Example.COM", &opts), "mc.example.com");
    }
}
