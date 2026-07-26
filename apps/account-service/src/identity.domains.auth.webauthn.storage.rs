use sqlx::{PgPool, Row};
use uuid::Uuid;
use webauthn_rs::prelude::{AuthenticationResult, DiscoverableKey, Passkey};

use crate::http::error::AppError;

#[derive(Clone)]
pub(crate) struct StoredPasskey {
    pub factor_id: Uuid,
    pub passkey: Passkey,
}

pub(crate) async fn load_passkeys(
    db: &PgPool,
    user_id: Uuid,
) -> Result<Vec<StoredPasskey>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, factor_data
        FROM mfa_factors
        WHERE principal_id = $1 AND factor_type = 'webauthn' AND status = 'active'
        ORDER BY created_at ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let factor_id: Uuid = row.get("id");
            let data: serde_json::Value = row.get("factor_data");
            data.get("passkey")
                .cloned()
                .and_then(|value| serde_json::from_value::<Passkey>(value).ok())
                .map(|passkey| StoredPasskey { factor_id, passkey })
        })
        .collect())
}

pub(crate) async fn persist_passkey(
    db: &PgPool,
    user_id: Uuid,
    factor_id: Uuid,
    passkey: &Passkey,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE mfa_factors
        SET factor_data = jsonb_set(COALESCE(factor_data, '{}'::jsonb), '{passkey}', $3::jsonb, true),
            last_used_at = NOW()
        WHERE id = $1
          AND principal_id = $2
          AND factor_type = 'webauthn'
          AND status = 'active'
          AND factor_data ? 'passkey'
        "#,
    )
    .bind(factor_id)
    .bind(user_id)
    .bind(sqlx::types::Json(passkey))
    .execute(db)
    .await?;

    Ok(())
}

pub(crate) async fn fetch_user_email(db: &PgPool, principal_id: Uuid) -> Result<String, AppError> {
    sqlx::query_scalar(
        r#"
        SELECT email
        FROM users
        WHERE principal_id = $1
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await
    .map_err(Into::into)
}

pub(crate) fn passkey_credentials(passkeys: &[StoredPasskey]) -> Vec<Passkey> {
    passkeys
        .iter()
        .map(|stored| stored.passkey.clone())
        .collect()
}

pub(crate) fn discoverable_keys(passkeys: &[StoredPasskey]) -> Vec<DiscoverableKey> {
    passkeys
        .iter()
        .map(|stored| DiscoverableKey::from(&stored.passkey))
        .collect()
}

pub(crate) async fn record_passkey_authentication(
    db: &PgPool,
    user_id: Uuid,
    passkeys: &mut [StoredPasskey],
    result: &AuthenticationResult,
) -> Result<(), AppError> {
    if !result.user_verified() {
        return Err(AppError::forbidden(
            "webauthn_user_not_verified",
            "WebAuthn authentication requires user verification.",
        ));
    }

    let stored = passkeys
        .iter_mut()
        .find(|stored| stored.passkey.cred_id() == result.cred_id())
        .ok_or_else(|| {
            AppError::forbidden(
                "webauthn_credential_not_found",
                "WebAuthn credential is not available for this account.",
            )
        })?;

    stored.passkey.update_credential(result);
    persist_passkey(db, user_id, stored.factor_id, &stored.passkey).await
}
