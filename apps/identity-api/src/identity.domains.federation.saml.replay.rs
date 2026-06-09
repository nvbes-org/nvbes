use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::http::error::AppError;

const ASSERTION_TTL_SECONDS: i64 = 300;

pub async fn check_assertion_replay(
    db: &PgPool,
    tenant_id: Uuid,
    provider_id: Uuid,
    assertion_id: &str,
) -> Result<bool, AppError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM saml_assertion_ids WHERE tenant_id = $1 AND provider_id = $2 AND assertion_id = $3)",
    )
    .bind(tenant_id)
    .bind(provider_id)
    .bind(assertion_id)
    .fetch_one(db)
    .await?;

    Ok(exists)
}

pub async fn record_assertion_id(
    db: &PgPool,
    tenant_id: Uuid,
    provider_id: Uuid,
    assertion_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO saml_assertion_ids (tenant_id, provider_id, assertion_id, processed_at) VALUES ($1, $2, $3, $4) ON CONFLICT (tenant_id, provider_id, assertion_id) DO NOTHING",
    )
    .bind(tenant_id)
    .bind(provider_id)
    .bind(assertion_id)
    .bind(Utc::now())
    .execute(db)
    .await?;

    Ok(())
}

pub async fn cleanup_expired_assertions(db: &PgPool) -> Result<u64, AppError> {
    let cutoff = Utc::now() - chrono::Duration::seconds(ASSERTION_TTL_SECONDS);

    let result = sqlx::query("DELETE FROM saml_assertion_ids WHERE processed_at < $1")
        .bind(cutoff)
        .execute(db)
        .await?;

    Ok(result.rows_affected())
}

pub async fn validate_and_record_assertion(
    db: &PgPool,
    tenant_id: Uuid,
    provider_id: Uuid,
    assertion_id: &str,
) -> Result<(), AppError> {
    if check_assertion_replay(db, tenant_id, provider_id, assertion_id).await? {
        return Err(AppError::bad_request(
            "saml_assertion_replay_detected",
            "This SAML assertion has already been processed.",
        ));
    }

    record_assertion_id(db, tenant_id, provider_id, assertion_id).await?;

    Ok(())
}

pub fn extract_assertion_id(xml: &str) -> Option<String> {
    let doc = roxmltree::Document::parse(xml).ok()?;

    for node in doc.descendants() {
        if node.is_element()
            && node.tag_name().name() == "Assertion"
            && let Some(id) = node.attribute("ID")
        {
            return Some(id.trim().to_string());
        }
    }

    None
}
