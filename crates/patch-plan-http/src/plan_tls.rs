//! Rustls TLS config for Plan HTTP (PEM + PKCS12).

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
    }
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
