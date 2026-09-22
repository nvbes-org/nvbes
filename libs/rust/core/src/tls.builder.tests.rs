use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Once,
};

use crate::config::AppConfig;

use super::{build_mtls_acceptor, build_mtls_identity};

static INSTALL_CRYPTO: Once = Once::new();

fn ensure_crypto_provider() {
    INSTALL_CRYPTO.call_once(|| {
        let _ = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

fn write_self_signed_material(dir: &Path) -> (PathBuf, PathBuf) {
    let key = dir.join("server.key");
    let cert = dir.join("server.crt");
    let output = Command::new("openssl")
        .args([
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-sha256",
            "-nodes",
            "-keyout",
            key.to_str().unwrap(),
            "-out",
            cert.to_str().unwrap(),
            "-days",
            "1",
            "-subj",
            "/CN=nvbes-test-mtls",
        ])
        .output()
        .expect("openssl available");
    assert!(
        output.status.success(),
        "openssl failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    (cert, key)
}

#[tokio::test]
async fn build_mtls_acceptor_requires_tls_paths() {
    ensure_crypto_provider();
    let config = AppConfig::default();
    let error = build_mtls_acceptor(&config)
        .await
        .expect_err("missing paths");
    assert!(error.contains("NVBES_TLS_CERT_PATH"));
}

#[tokio::test]
async fn build_mtls_acceptor_loads_local_pem_files() {
    ensure_crypto_provider();
    let dir = tempfile_dir();
    let (cert, key) = write_self_signed_material(&dir);
    let config = AppConfig {
        tls_cert_path: Some(cert.to_string_lossy().into_owned()),
        tls_key_path: Some(key.to_string_lossy().into_owned()),
        tls_client_ca_path: Some(cert.to_string_lossy().into_owned()),
        ..AppConfig::default()
    };
    build_mtls_acceptor(&config)
        .await
        .expect("acceptor builds from local PEMs");
}

#[test]
fn build_mtls_identity_reads_combined_pem() {
    ensure_crypto_provider();
    let dir = tempfile_dir();
    let (cert, key) = write_self_signed_material(&dir);
    build_mtls_identity(cert.to_str().unwrap(), key.to_str().unwrap()).expect("client identity");
}

#[test]
fn build_mtls_identity_rejects_missing_files() {
    let error =
        build_mtls_identity("/no/such/cert.pem", "/no/such/key.pem").expect_err("missing files");
    assert!(error.contains("Failed to read client cert"));
}

#[test]
fn build_mtls_identity_rejects_invalid_pem() {
    let dir = tempfile_dir();
    let cert = dir.join("bad.crt");
    let key = dir.join("bad.key");
    fs::write(&cert, "not-a-cert").expect("write");
    fs::write(&key, "not-a-key").expect("write");
    let error = build_mtls_identity(cert.to_str().unwrap(), key.to_str().unwrap())
        .expect_err("invalid pem");
    assert!(error.contains("Failed to parse client identity"));
}

fn tempfile_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "nvbes-tls-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}
