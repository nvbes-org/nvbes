use sqlx::PgPool;
use uuid::Uuid;

use crate::{auth::audit, mfa_crypto::MfaCrypto};

pub async fn rotate(db: &PgPool, crypto: &MfaCrypto) -> anyhow::Result<u64> {
    let mut tx = db.begin().await?;
    let factors = sqlx::query_as::<_, (Uuid, Uuid, Vec<u8>, Vec<u8>, i16)>(
        "SELECT id, principal_id, secret_ciphertext, secret_nonce, key_version FROM identity_auth_factors WHERE key_version <> $1 FOR UPDATE",
    )
    .bind(crypto.active_version())
    .fetch_all(&mut *tx)
    .await?;

    for (factor_id, principal_id, ciphertext, nonce, key_version) in &factors {
        let secret = crypto.open(*factor_id, *key_version, ciphertext, nonce)?;
        let sealed = crypto.seal(*factor_id, &secret)?;
        sqlx::query(
            "UPDATE identity_auth_factors SET secret_ciphertext = $1, secret_nonce = $2, key_version = $3 WHERE id = $4",
        )
        .bind(sealed.ciphertext)
        .bind(sealed.nonce.as_slice())
        .bind(sealed.key_version)
        .bind(factor_id)
        .execute(&mut *tx)
        .await?;
        audit(&mut tx, *principal_id, "identity.mfa_key_rotated").await?;
    }

    tx.commit().await?;
    Ok(factors.len() as u64)
}
