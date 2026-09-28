use std::sync::Arc;

use crate::config::AppConfig;
use tokio_rustls::rustls;
use tokio_rustls::rustls::RootCertStore;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use tokio_rustls::rustls::server::WebPkiClientVerifier;

fn load_certs(path: &str) -> Result<Vec<CertificateDer<'static>>, String> {
    CertificateDer::pem_file_iter(path)
        .map_err(|e| format!("Failed to read certs from {path}: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to parse certs from {path}: {e}"))
}

fn load_private_key(path: &str) -> Result<PrivateKeyDer<'static>, String> {
    PrivateKeyDer::from_pem_file(path)
        .map_err(|e| format!("Failed to parse private key from {path}: {e}"))
}

pub async fn build_mtls_acceptor(
    config: &AppConfig,
) -> Result<axum_server::tls_rustls::RustlsConfig, String> {
    let cert_path = config
        .tls_cert_path
        .as_deref()
        .ok_or("NVBES_TLS_CERT_PATH is required for mTLS")?;
    let key_path = config
        .tls_key_path
        .as_deref()
        .ok_or("NVBES_TLS_KEY_PATH is required for mTLS")?;
    let ca_path = config
        .tls_client_ca_path
        .as_deref()
        .ok_or("NVBES_TLS_CLIENT_CA_PATH is required for mTLS")?;

    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;

    let mut root_store = RootCertStore::empty();
    let ca_certs = load_certs(ca_path)?;
    for (i, cert) in ca_certs.iter().enumerate() {
        root_store
            .add(cert.clone())
            .map_err(|e| format!("Failed to add CA cert #{i}: {e}"))?;
    }

    let verifier = WebPkiClientVerifier::builder(Arc::new(root_store))
        .build()
        .map_err(|e| format!("Failed to build client cert verifier: {e}"))?;

    let server_config = rustls::ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(certs, key)
        .map_err(|e| format!("Failed to build TLS server config: {e}"))?;

    Ok(axum_server::tls_rustls::RustlsConfig::from_config(
        server_config.into(),
    ))
}

pub fn build_mtls_identity(cert_path: &str, key_path: &str) -> Result<reqwest::Identity, String> {
    let cert_pem = std::fs::read_to_string(cert_path)
        .map_err(|e| format!("Failed to read client cert {cert_path}: {e}"))?;
    let key_pem = std::fs::read_to_string(key_path)
        .map_err(|e| format!("Failed to read client key {key_path}: {e}"))?;
    let combined = format!("{cert_pem}\n{key_pem}");
    reqwest::Identity::from_pem(combined.as_bytes())
        .map_err(|e| format!("Failed to parse client identity: {e}"))
}

#[cfg(test)]
#[path = "tls.builder.tests.rs"]
mod tests;
