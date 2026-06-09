use base64::Engine;
use openssl::pkey::{PKey, Public};
use sqlx::PgPool;

use nvbes_core::scw_kms::{KmsClient, KmsError};

use crate::http::error::AppError;

#[derive(Debug, Clone)]
pub struct SigningKey {
    pub kid: String,
    pub kms_key_id: Option<String>,
    public_key_pem: String,
    pub status: String,
    pub activated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct JwkEntry {
    pub kid: String,
    pub n: String,
    pub e: String,
}

type SigningKeyRow = (
    String,
    Option<String>,
    String,
    String,
    chrono::DateTime<chrono::Utc>,
);

pub enum KeyBackend {
    Kms(KmsClient),
    Local,
}

impl KeyBackend {
    pub fn is_kms(&self) -> bool {
        matches!(self, Self::Kms(_))
    }

    pub fn kms_client(&self) -> Option<&KmsClient> {
        match self {
            Self::Kms(c) => Some(c),
            Self::Local => None,
        }
    }
}

pub async fn create_initial_key(pool: &PgPool, backend: &KeyBackend) -> Result<(), AppError> {
    let existing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM signing_keys")
        .fetch_one(pool)
        .await?;
    if existing > 0 {
        return Ok(());
    }

    match backend {
        KeyBackend::Kms(client) => {
            let kms_key = client
                .create_key("nvbes-jwt-signing")
                .await
                .map_err(|e| AppError::internal("kms_key_creation_failed", e.to_string()))?;

            let public_key_resp = client
                .get_public_key(&kms_key.id)
                .await
                .map_err(|e| AppError::internal("kms_public_key_fetch_failed", e.to_string()))?;

            sqlx::query(
                "INSERT INTO signing_keys (kid, kms_key_id, public_key_pem, status) VALUES ($1, $2, $3, 'active')",
            )
            .bind(&kms_key.id)
            .bind(&kms_key.id)
            .bind(&public_key_resp.public_key)
            .execute(pool)
            .await?;
        }
        KeyBackend::Local => {
            let (kid, public_pem) = generate_local_key_pair()?;
            sqlx::query(
                "INSERT INTO signing_keys (kid, public_key_pem, status) VALUES ($1, $2, 'active')",
            )
            .bind(&kid)
            .bind(&public_pem)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

pub async fn rotate_key(pool: &PgPool, backend: &KeyBackend) -> Result<SigningKey, AppError> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        "UPDATE signing_keys SET deprecated_at = NOW(), status = 'deprecated' WHERE status = 'active'",
    )
    .execute(&mut *tx)
    .await?;

    let (kid, kms_key_id, public_pem) = match backend {
        KeyBackend::Kms(client) => {
            let active = get_active_key(pool)
                .await?
                .ok_or_else(|| AppError::internal("no_active_key", "No active key to rotate"))?;

            let kms_key_id = active.kms_key_id.as_ref().ok_or_else(|| {
                AppError::internal("key_not_kms_backed", "Active key is not KMS-backed")
            })?;

            let rotated = client
                .rotate_key(kms_key_id)
                .await
                .map_err(|e| AppError::internal("kms_rotation_failed", e.to_string()))?;

            let public_key_resp = client
                .get_public_key(kms_key_id)
                .await
                .map_err(|e| AppError::internal("kms_public_key_fetch_failed", e.to_string()))?;

            (
                format!("kid-{}", rotated.rotation_count),
                Some(kms_key_id.clone()),
                public_key_resp.public_key,
            )
        }
        KeyBackend::Local => {
            let (kid, public_pem) = generate_local_key_pair()?;
            (kid, None, public_pem)
        }
    };

    let row: SigningKeyRow = sqlx::query_as(
        "INSERT INTO signing_keys (kid, kms_key_id, public_key_pem, status) VALUES ($1, $2, $3, 'active') RETURNING kid, kms_key_id, public_key_pem, status, activated_at",
    )
    .bind(&kid)
    .bind(&kms_key_id)
    .bind(&public_pem)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(SigningKey {
        kid: row.0,
        kms_key_id: row.1,
        public_key_pem: row.2,
        status: row.3,
        activated_at: row.4,
    })
}

pub async fn get_active_key(pool: &PgPool) -> Result<Option<SigningKey>, AppError> {
    let row: Option<SigningKeyRow> =
        sqlx::query_as(
            "SELECT kid, kms_key_id, public_key_pem, status, activated_at FROM signing_keys WHERE status = 'active' ORDER BY activated_at DESC LIMIT 1",
        )
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| SigningKey {
        kid: r.0,
        kms_key_id: r.1,
        public_key_pem: r.2,
        status: r.3,
        activated_at: r.4,
    }))
}

pub async fn get_keys_for_jwks(pool: &PgPool) -> Result<Vec<JwkEntry>, AppError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT kid, public_key_pem FROM signing_keys WHERE status IN ('active', 'deprecated') AND (deprecated_at IS NULL OR deprecated_at > NOW() - INTERVAL '48 hours') ORDER BY activated_at DESC",
    )
    .fetch_all(pool)
    .await?;

    let entries: Vec<JwkEntry> = rows
        .into_iter()
        .filter_map(|(kid, public_key_pem)| {
            let public_key = PKey::public_key_from_pem(public_key_pem.as_bytes()).ok()?;
            let public_key = public_key.rsa().ok()?;
            let n =
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(public_key.n().to_vec());
            let e =
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(public_key.e().to_vec());
            Some(JwkEntry { kid, n, e })
        })
        .collect();

    Ok(entries)
}

pub async fn get_public_key_pems_for_decoding(
    pool: &PgPool,
) -> Result<Vec<(String, String)>, AppError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT kid, public_key_pem FROM signing_keys WHERE status IN ('active', 'deprecated') AND (deprecated_at IS NULL OR deprecated_at > NOW() - INTERVAL '48 hours') ORDER BY activated_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|(kid, public_key_pem)| {
            normalize_public_key_pem(&public_key_pem).map(|pem| (kid, pem))
        })
        .collect())
}

pub fn generate_local_key_pair() -> Result<(String, String), AppError> {
    use sha2::Digest;

    let private_key = openssl::rsa::Rsa::generate(4096)
        .map_err(|e| AppError::internal("key_generation_failed", e.to_string()))?;
    let key = openssl::pkey::PKey::from_rsa(private_key)
        .map_err(|e| AppError::internal("key_encoding_failed", e.to_string()))?;

    let public_pem = key
        .public_key_to_pem()
        .map_err(|e| AppError::internal("key_encoding_failed", e.to_string()))?;

    let der = key
        .public_key_to_der()
        .map_err(|e| AppError::internal("key_der_failed", e.to_string()))?;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&der);
    let kid = format!("kid-local-{}", hex::encode(&hasher.finalize()[..8]));

    let public_pem = String::from_utf8(public_pem)
        .map_err(|e| AppError::internal("key_encoding_failed", e.to_string()))?;

    Ok((kid, public_pem))
}

fn normalize_public_key_pem(public_key_pem: &str) -> Option<String> {
    let key: PKey<Public> = PKey::public_key_from_pem(public_key_pem.as_bytes()).ok()?;
    String::from_utf8(key.public_key_to_pem().ok()?).ok()
}

impl SigningKey {
    pub fn public_key_pem(&self) -> &str {
        &self.public_key_pem
    }
}

impl From<KmsError> for AppError {
    fn from(e: KmsError) -> Self {
        AppError::internal("kms_error", e.to_string())
    }
}
