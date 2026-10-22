//! Plan HTTPS material discovery (indepth.md §5.2 — PEM + PKCS12).

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum PlanTlsMaterial {
    Pem { cert: PathBuf, key: PathBuf },
    Pkcs12 { path: PathBuf, password: String },
}

/// Resolves TLS config: PEM pair first, then `plan/tls/keystore.p12` or `cert.p12`.
pub fn resolve_plan_tls_material(network_data_path: &Path) -> Option<PlanTlsMaterial> {
    if let Some((cert, key)) = crate::plan_export::plan_tls_pem_paths(network_data_path) {
        return Some(PlanTlsMaterial::Pem { cert, key });
    }
    let dir = network_data_path.join("plan/tls");
    for name in ["keystore.p12", "cert.p12", "plan.p12"] {
        let path = dir.join(name);
        if path.is_file() {
            let password = std::env::var("PATCH_PLAN_TLS_PKCS12_PASSWORD").unwrap_or_default();
            return Some(PlanTlsMaterial::Pkcs12 { path, password });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_pem_over_pkcs12() {
        let tmp = tempfile::tempdir().unwrap();
        let tls = tmp.path().join("plan/tls");
        std::fs::create_dir_all(&tls).unwrap();
        std::fs::write(tls.join("cert.pem"), b"x").unwrap();
        std::fs::write(tls.join("key.pem"), b"y").unwrap();
        std::fs::write(tls.join("keystore.p12"), b"z").unwrap();
        let material = resolve_plan_tls_material(tmp.path()).unwrap();
        assert!(matches!(material, PlanTlsMaterial::Pem { .. }));
    }
}
