use sqlx::PgPool;

use crate::{auth::audit, mfa_crypto::MfaCrypto};

pub async fn rotate(db: &PgPool, crypto: &MfaCrypto) -> anyhow::Result<u64> {
    let mut tx = db.begin().await?;
    let factors = sqlx::query!(
        "SELECT id, principal_id, secret_ciphertext, secret_nonce, key_version FROM identity_auth_factors WHERE key_version <> $1 FOR UPDATE",
        crypto.active_version()
    )
    .fetch_all(&mut *tx)
    .await?;

    for factor in &factors {
        let secret = crypto.open(
            factor.id,
            factor.key_version,
            &factor.secret_ciphertext,
            &factor.secret_nonce,
        )?;
        let sealed = crypto.seal(factor.id, &secret)?;
        sqlx::query!(
            "UPDATE identity_auth_factors SET secret_ciphertext = $1, secret_nonce = $2, key_version = $3 WHERE id = $4",
            sealed.ciphertext,
            sealed.nonce.as_slice(),
            sealed.key_version,
            factor.id
        )
        .execute(&mut *tx)
        .await?;
        audit(&mut tx, factor.principal_id, "identity.mfa_key_rotated").await?;
    }

    tx.commit().await?;
    Ok(factors.len() as u64)
}
