use sqlx::{PgPool, Row};
use uuid::Uuid;
use webauthn_rs::prelude::Passkey;

use crate::http::error::AppError;

pub(crate) async fn load_passkeys(db: &PgPool, user_id: Uuid) -> Result<Vec<Passkey>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT factor_data
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
            let data: serde_json::Value = row.get("factor_data");
            data.get("passkey")
                .cloned()
                .and_then(|value| serde_json::from_value::<Passkey>(value).ok())
        })
        .collect())
}

pub(crate) async fn persist_passkey(
    db: &PgPool,
    user_id: Uuid,
    passkey: &Passkey,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE mfa_factors
        SET factor_data = jsonb_set(COALESCE(factor_data, '{}'::jsonb), '{passkey}', $2::jsonb, true),
            last_used_at = NOW()
        WHERE principal_id = $1 AND factor_type = 'webauthn' AND status = 'active'
          AND factor_data ? 'passkey'
        "#,
    )
    .bind(user_id)
    .bind(sqlx::types::Json(passkey))
    .execute(db)
    .await?;

    Ok(())
}
