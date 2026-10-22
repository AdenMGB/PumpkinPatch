//! Rustls TLS config for Plan HTTP (PEM + PKCS12 + JKS).

use patch_core::PlanTlsMaterial;
use std::io;
use std::path::Path;

pub async fn rustls_config_from(
    material: &PlanTlsMaterial,
) -> Result<axum_server::tls_rustls::RustlsConfig, io::Error> {
    match material {
        PlanTlsMaterial::Pem { cert, key } => {
            axum_server::tls_rustls::RustlsConfig::from_pem_file(cert.clone(), key.clone())
                .await
                .map_err(io::Error::other)
        }
        PlanTlsMaterial::Pkcs12 { path, password } => {
            let (cert_pem, key_pem) = pkcs12_to_pem(path, password)?;
            axum_server::tls_rustls::RustlsConfig::from_pem(cert_pem, key_pem)
                .await
                .map_err(io::Error::other)
        }
        PlanTlsMaterial::Jks { path, password, alias } => {
            let (cert_pem, key_pem) = jks_to_pem(path, password, alias.as_deref())?;
            axum_server::tls_rustls::RustlsConfig::from_pem(cert_pem, key_pem)
                .await
                .map_err(io::Error::other)
        }
    }
}

fn jks_to_pem(
    path: &Path,
    password: &str,
    alias: Option<&str>,
) -> Result<(Vec<u8>, Vec<u8>), io::Error> {
    use jks::KeyStore;

    let data = std::fs::read(path)?;
    let mut ks = KeyStore::new();
    ks.load(&mut data.as_slice(), password.as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    let pick_alias = alias
        .map(|a| a.to_string())
        .or_else(|| {
            ks.aliases()
                .into_iter()
                .find(|a| ks.is_private_key_entry(a))
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "JKS keystore has no private key entry (set PATCH_PLAN_TLS_JKS_ALIAS)",
            )
        })?;

    let entry = ks
        .get_private_key_entry(&pick_alias, password.as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    let key = openssl::pkey::PKey::private_key_from_der(&entry.private_key)
        .map_err(io::Error::other)?;
    let key_pem = key.private_key_to_pem_pkcs8().map_err(io::Error::other)?;

    let mut cert_pem = Vec::new();
    for cert in &entry.certificate_chain {
        let x509 = openssl::x509::X509::from_der(&cert.content).map_err(io::Error::other)?;
        cert_pem.extend(x509.to_pem().map_err(io::Error::other)?);
    }
    if cert_pem.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "JKS entry missing certificate chain",
        ));
    }
    Ok((cert_pem, key_pem))
}

fn pkcs12_to_pem(path: &Path, password: &str) -> Result<(Vec<u8>, Vec<u8>), io::Error> {
    let der = std::fs::read(path)?;
    let pkcs12 = openssl::pkcs12::Pkcs12::from_der(&der).map_err(io::Error::other)?;
    let parsed = pkcs12.parse2(password).map_err(io::Error::other)?;
    let cert = parsed
        .cert
        .as_ref()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "pkcs12 missing certificate"))?;
    let key = parsed
        .pkey
        .as_ref()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "pkcs12 missing private key"))?;
    let cert_pem = cert.to_pem().map_err(io::Error::other)?;
    let key_pem = key.private_key_to_pem_pkcs8().map_err(io::Error::other)?;
    Ok((cert_pem, key_pem))
}
