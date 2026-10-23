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

fn der_to_pem(label: &str, der: &[u8]) -> Vec<u8> {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(der);
    let mut pem = format!("-----BEGIN {label}-----\n");
    for line in b64.as_bytes().chunks(64) {
        pem.push_str(&String::from_utf8_lossy(line));
        pem.push('\n');
    }
    pem.push_str(&format!("-----END {label}-----\n"));
    pem.into_bytes()
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

    let key_pem = der_to_pem("PRIVATE KEY", &entry.private_key);

    let mut cert_pem = Vec::new();
    for cert in &entry.certificate_chain {
        cert_pem.extend(der_to_pem("CERTIFICATE", &cert.content));
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
    let pfx = p12::PFX::parse(&der).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("invalid PKCS#12: {e}"))
    })?;
    let keys = pfx.key_bags(password).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("PKCS#12 decrypt failed (wrong password?): {e}"),
        )
    })?;
    let key_der = keys
        .first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "pkcs12 missing private key"))?;
    let certs = pfx.cert_x509_bags(password).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("PKCS#12 cert bags: {e}"))
    })?;
    let first_cert = certs
        .first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "pkcs12 missing certificate"))?;
    let mut cert_pem = der_to_pem("CERTIFICATE", first_cert);
    for extra in certs.iter().skip(1) {
        cert_pem.extend(der_to_pem("CERTIFICATE", extra));
    }
    let key_pem = der_to_pem("PRIVATE KEY", key_der);
    Ok((cert_pem, key_pem))
}
