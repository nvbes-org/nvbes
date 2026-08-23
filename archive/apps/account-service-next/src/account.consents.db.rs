use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    consents_models::{AccountConsent, ConsentHistory, ConsentInput},
    error::AppError,
};

pub async fn grant(
    db: &PgPool,
    principal_id: Uuid,
    input: ConsentInput,
    ip_address: Option<String>,
) -> Result<AccountConsent, AppError> {
    sqlx::query_as::<_, AccountConsent>(
        r#"
        INSERT INTO account_consents
          (id, principal_id, consent_type, document_version, ip_address)
        VALUES ($1, $2, $3, $4, $5::inet)
        ON CONFLICT (principal_id, consent_type, document_version)
          WHERE revoked_at IS NULL
        DO UPDATE SET granted_at = NOW(), ip_address = EXCLUDED.ip_address
        RETURNING id, principal_id, consent_type, document_version,
          host(ip_address) AS ip_address, granted_at, revoked_at
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(principal_id)
    .bind(input.consent_type)
    .bind(input.document_version)
    .bind(ip_address)
    .fetch_one(db)
    .await
    .map_err(AppError::from)
}

pub async fn revoke(db: &PgPool, principal_id: Uuid, input: ConsentInput) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE account_consents
        SET revoked_at = NOW()
        WHERE principal_id = $1
          AND consent_type = $2
          AND document_version = $3
          AND revoked_at IS NULL
        "#,
    )
    .bind(principal_id)
    .bind(input.consent_type)
    .bind(input.document_version)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn list(
    db: &PgPool,
    principal_id: Uuid,
    cursor: Option<Uuid>,
    limit: i64,
) -> Result<ConsentHistory, AppError> {
    let mut rows = sqlx::query_as::<_, AccountConsent>(
        r#"
        SELECT id, principal_id, consent_type, document_version,
          host(ip_address) AS ip_address, granted_at, revoked_at
        FROM account_consents
        WHERE principal_id = $1
          AND (
            $2::uuid IS NULL
            OR (granted_at, id) < (
              SELECT granted_at, id
              FROM account_consents
              WHERE principal_id = $1 AND id = $2
            )
          )
        ORDER BY granted_at DESC, id DESC
        LIMIT $3
        "#,
    )
    .bind(principal_id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(db)
    .await?;
    let has_more = rows.len() > usize::try_from(limit).unwrap_or(100);
    if has_more {
        rows.pop();
    }
    let next_cursor = has_more
        .then(|| rows.last().map(|row| row.id.to_string()))
        .flatten();
    Ok(ConsentHistory {
        consents: rows,
        next_cursor,
        has_more,
    })
}
