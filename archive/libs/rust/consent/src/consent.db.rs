use chrono::Utc;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ConsentError;
use crate::models::{CookiePreferences, LegalAgreement};

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub async fn insert_legal_agreement(
    pool: &PgPool,
    agreement: &LegalAgreement,
) -> Result<LegalAgreement, ConsentError> {
    let row = sqlx::query_as::<_, LegalAgreement>(
        // nvbes-allow-runtime-sql
        r#"
        INSERT INTO consent.legal_agreements (
            id, subject_id, document_type, document_version, agreed_at, ip_hash, user_agent
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, subject_id, document_type, document_version, agreed_at, ip_hash, user_agent
        "#,
    )
    .bind(agreement.id)
    .bind(agreement.subject_id)
    .bind(&agreement.document_type)
    .bind(&agreement.document_version)
    .bind(agreement.agreed_at)
    .bind(&agreement.ip_hash)
    .bind(&agreement.user_agent)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn has_accepted_version(
    pool: &PgPool,
    subject_id: Uuid,
    document_type: &str,
    required_version: &str,
) -> Result<bool, ConsentError> {
    let accepted = sqlx::query_scalar::<_, bool>(
        // nvbes-allow-runtime-sql
        r#"
        SELECT EXISTS (
            SELECT 1 FROM consent.legal_agreements
            WHERE subject_id = $1 AND document_type = $2 AND document_version = $3
        )
        "#,
    )
    .bind(subject_id)
    .bind(document_type)
    .bind(required_version)
    .fetch_one(pool)
    .await?;

    Ok(accepted)
}

pub async fn get_latest_legal_agreements(
    pool: &PgPool,
    subject_id: Uuid,
) -> Result<Vec<LegalAgreement>, ConsentError> {
    let rows = sqlx::query_as::<_, LegalAgreement>(
        // nvbes-allow-runtime-sql
        r#"
        SELECT DISTINCT ON (document_type)
            id, subject_id, document_type, document_version, agreed_at, ip_hash, user_agent
        FROM consent.legal_agreements
        WHERE subject_id = $1
        ORDER BY document_type, agreed_at DESC
        "#,
    )
    .bind(subject_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn get_cookie_preferences(
    pool: &PgPool,
    subject_id: Uuid,
) -> Result<Option<CookiePreferences>, ConsentError> {
    let row = sqlx::query_as::<_, CookiePreferences>(
        // nvbes-allow-runtime-sql
        r#"
        SELECT id, subject_id, functional, analytics, marketing, gpc_signal, updated_at
        FROM consent.cookie_preferences
        WHERE subject_id = $1
        "#,
    )
    .bind(subject_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn upsert_cookie_preferences(
    pool: &PgPool,
    preferences: &CookiePreferences,
) -> Result<CookiePreferences, ConsentError> {
    let row = sqlx::query_as::<_, CookiePreferences>(
        // nvbes-allow-runtime-sql
        r#"
        INSERT INTO consent.cookie_preferences (
            id, subject_id, functional, analytics, marketing, gpc_signal, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (subject_id) DO UPDATE
        SET analytics = EXCLUDED.analytics,
            marketing = EXCLUDED.marketing,
            gpc_signal = EXCLUDED.gpc_signal,
            updated_at = EXCLUDED.updated_at
        WHERE consent.cookie_preferences.subject_id = EXCLUDED.subject_id
        RETURNING id, subject_id, functional, analytics, marketing, gpc_signal, updated_at
        "#,
    )
    .bind(preferences.id)
    .bind(preferences.subject_id)
    .bind(preferences.functional)
    .bind(preferences.analytics)
    .bind(preferences.marketing)
    .bind(preferences.gpc_signal)
    .bind(preferences.updated_at)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn insert_audit_event(
    pool: &PgPool,
    subject_id: Uuid,
    action: &str,
    details: &Value,
) -> Result<(), ConsentError> {
    sqlx::query(
        // nvbes-allow-runtime-sql
        r#"
        INSERT INTO consent.audit_events (id, subject_id, action, details, created_at)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(subject_id)
    .bind(action)
    .bind(details)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn reconcile_subject(
    pool: &PgPool,
    anonymous_id: Uuid,
    authenticated_id: Uuid,
) -> Result<(), ConsentError> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        // nvbes-allow-runtime-sql
        r#"
        UPDATE consent.legal_agreements
        SET subject_id = $2
        WHERE subject_id = $1
        "#,
    )
    .bind(anonymous_id)
    .bind(authenticated_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        // nvbes-allow-runtime-sql
        r#"
        UPDATE consent.cookie_preferences
        SET subject_id = $2
        WHERE subject_id = $1
          AND NOT EXISTS (
              SELECT 1 FROM consent.cookie_preferences cp2
              WHERE cp2.subject_id = $2
          )
        "#,
    )
    .bind(anonymous_id)
    .bind(authenticated_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        // nvbes-allow-runtime-sql
        r#"
        DELETE FROM consent.cookie_preferences
        WHERE subject_id = $1
        "#,
    )
    .bind(anonymous_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}
