use std::fs;
use std::path::{Path, PathBuf};

use openssl::pkey::{PKey, Private, Public};
use sha2::Digest;
use sqlx::PgPool;

use crate::http::error::AppError;

#[derive(Debug, Clone)]
pub struct LocalSigningKeyMaterial {
    pub kid: String,
    pub private_key_pem: String,
    pub public_key_pem: String,
}

pub fn load_or_create_local_signing_key_material() -> Result<LocalSigningKeyMaterial, AppError> {
    if std::env::var("NVBES_ENV").ok().as_deref() != Some("development") {
        return Err(AppError::internal(
            "local_signing_key_forbidden",
            "Local JWT signing keys are only allowed in development.",
        ));
    }

    let private_key_path = local_signing_key_path()?;

    let private_key_pem = if private_key_path.exists() {
        fs::read_to_string(&private_key_path)
            .map_err(|e| AppError::internal("local_signing_key_read_failed", e.to_string()))?
    } else {
        let private_key_pem = generate_local_private_key_pem()?;
        persist_local_signing_key(&private_key_path, &private_key_pem)?;
        private_key_pem
    };

    material_from_private_key_pem(&private_key_pem)
}

pub async fn ensure_local_signing_key(
    pool: &PgPool,
    material: &LocalSigningKeyMaterial,
) -> Result<(), AppError> {
    let active: Option<(String, String)> = sqlx::query_as(
        "SELECT kid, public_key_pem FROM signing_keys WHERE status = 'active' ORDER BY activated_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    if let Some((active_kid, active_public_key_pem)) = active {
        if active_kid == material.kid
            && normalize_public_key_pem(&active_public_key_pem).as_deref()
                == Some(material.public_key_pem.as_str())
        {
            return Ok(());
        }

        sqlx::query(
            "UPDATE signing_keys SET status = 'deprecated', deprecated_at = NOW() WHERE status = 'active'",
        )
        .execute(pool)
        .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO signing_keys (kid, public_key_pem, status)
        VALUES ($1, $2, 'active')
        ON CONFLICT (kid) DO UPDATE
        SET public_key_pem = EXCLUDED.public_key_pem,
            status = 'active',
            deprecated_at = NULL,
            revoked_at = NULL
        "#,
    )
    .bind(&material.kid)
    .bind(&material.public_key_pem)
    .execute(pool)
    .await?;

    Ok(())
}

fn local_signing_key_path() -> Result<PathBuf, AppError> {
    let root = std::env::var_os("NVBES_WORKSPACE_ROOT")
        .map(PathBuf::from)
        .unwrap_or(
            std::env::current_dir()
                .map_err(|e| AppError::internal("workspace_root_unavailable", e.to_string()))?,
        );

    Ok(root.join(".temp/account-service/jwt/private_key.pem"))
}

fn persist_local_signing_key(path: &Path, private_key_pem: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AppError::internal("local_signing_key_dir_failed", e.to_string()))?;
    }

    fs::write(path, private_key_pem)
        .map_err(|e| AppError::internal("local_signing_key_write_failed", e.to_string()))
}

fn generate_local_private_key_pem() -> Result<String, AppError> {
    let private_key = openssl::rsa::Rsa::generate(4096)
        .map_err(|e| AppError::internal("key_generation_failed", e.to_string()))?;
    let key = PKey::from_rsa(private_key)
        .map_err(|e| AppError::internal("key_encoding_failed", e.to_string()))?;
    let pem = key
        .private_key_to_pem_pkcs8()
        .map_err(|e| AppError::internal("key_encoding_failed", e.to_string()))?;
    String::from_utf8(pem).map_err(|e| AppError::internal("key_encoding_failed", e.to_string()))
}

fn material_from_private_key_pem(
    private_key_pem: &str,
) -> Result<LocalSigningKeyMaterial, AppError> {
    let private_key: PKey<Private> = PKey::private_key_from_pem(private_key_pem.as_bytes())
        .map_err(|e| AppError::internal("key_loading_failed", e.to_string()))?;

    let public_pem = private_key
        .public_key_to_pem()
        .map_err(|e| AppError::internal("key_encoding_failed", e.to_string()))?;
    let public_key_pem = String::from_utf8(public_pem)
        .map_err(|e| AppError::internal("key_encoding_failed", e.to_string()))?;

    let der = private_key
        .public_key_to_der()
        .map_err(|e| AppError::internal("key_der_failed", e.to_string()))?;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&der);
    let kid = format!("kid-local-{}", hex::encode(&hasher.finalize()[..8]));

    Ok(LocalSigningKeyMaterial {
        kid,
        private_key_pem: private_key_pem.to_string(),
        public_key_pem,
    })
}

fn normalize_public_key_pem(public_key_pem: &str) -> Option<String> {
    let key: PKey<Public> = PKey::public_key_from_pem(public_key_pem.as_bytes()).ok()?;
    String::from_utf8(key.public_key_to_pem().ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_public_key_and_kid_from_private_key() {
        let private_key = openssl::rsa::Rsa::generate(2048).expect("rsa");
        let key = PKey::from_rsa(private_key).expect("pkey");
        let pem = String::from_utf8(
            key.private_key_to_pem_pkcs8()
                .expect("private pem should encode"),
        )
        .expect("pem utf8");

        let material = material_from_private_key_pem(&pem).expect("material");

        assert!(material.kid.starts_with("kid-local-"));
        assert!(!material.public_key_pem.is_empty());
        assert_eq!(material.private_key_pem, pem);
    }
}
