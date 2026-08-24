use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    audit::{self, AuditEvent},
    config::RetentionConfig,
};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RetentionResult {
    pub signals: u64,
    pub features: u64,
    pub evaluations: u64,
    pub labels: u64,
    pub reviews: u64,
    pub audit: u64,
}

pub async fn apply(pool: &PgPool) -> Result<RetentionResult, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let signals = sqlx::query("DELETE FROM trust_risk_signals WHERE projected_at IS NOT NULL AND retention_deadline <= clock_timestamp() AND NOT legal_hold")
        .execute(&mut *tx).await?.rows_affected();
    let features =
        sqlx::query("DELETE FROM trust_risk_feature_state WHERE expires_at <= clock_timestamp()")
            .execute(&mut *tx)
            .await?
            .rows_affected();
    let labels = sqlx::query("DELETE FROM trust_risk_labels AS label WHERE expires_at <= clock_timestamp() AND NOT legal_hold AND (review_case_id IS NULL OR NOT EXISTS (SELECT 1 FROM trust_risk_review_cases AS review WHERE review.id = label.review_case_id AND review.legal_hold))")
        .execute(&mut *tx).await?.rows_affected();
    let reviews = sqlx::query("DELETE FROM trust_risk_review_cases WHERE expires_at <= clock_timestamp() AND NOT legal_hold")
        .execute(&mut *tx).await?.rows_affected();
    let evaluations = sqlx::query("DELETE FROM trust_risk_evaluations WHERE expires_at <= clock_timestamp() AND NOT legal_hold AND NOT EXISTS (SELECT 1 FROM trust_risk_labels WHERE evaluation_id = trust_risk_evaluations.id) AND NOT EXISTS (SELECT 1 FROM trust_risk_review_cases WHERE evaluation_id = trust_risk_evaluations.id)")
        .execute(&mut *tx).await?.rows_affected();
    let audit_rows = sqlx::query("DELETE FROM trust_risk_audit_events WHERE expires_at <= clock_timestamp() AND NOT legal_hold")
        .execute(&mut *tx).await?.rows_affected();
    tx.commit().await?;
    Ok(RetentionResult {
        signals,
        features,
        evaluations,
        labels,
        reviews,
        audit: audit_rows,
    })
}

pub async fn erase_subject(
    pool: &PgPool,
    kind: i16,
    namespace: &str,
    opaque_id: &str,
    actor: &str,
    reason: &str,
    retention: RetentionConfig,
) -> Result<Uuid, ErasureError> {
    if !(1..=6).contains(&kind)
        || namespace.len() < 3
        || opaque_id.len() < 8
        || actor.len() < 3
        || reason.trim().len() < 3
    {
        return Err(ErasureError::InvalidInput);
    }
    let request_id = Uuid::new_v4();
    let audit_target = request_id.to_string();
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM trust_risk_signal_subjects WHERE kind = $1 AND namespace = $2 AND opaque_id = $3")
        .bind(kind).bind(namespace).bind(opaque_id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM trust_risk_feature_state WHERE subject_kind = $1 AND namespace = $2 AND opaque_id = $3")
        .bind(kind).bind(namespace).bind(opaque_id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO trust_risk_rebuild_requests (id, subject_kind, namespace, opaque_id, reason, requested_by) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(request_id).bind(kind).bind(namespace).bind(opaque_id).bind(reason.trim()).bind(actor)
        .execute(&mut *tx).await?;
    audit::append(
        &mut tx,
        AuditEvent {
            action: "subject.erase",
            actor,
            reason,
            target_type: "subject",
            target_id: &audit_target,
            detail: serde_json::json!({"kind": kind, "namespace": namespace}),
        },
        retention.audit_days,
    )
    .await?;
    tx.commit().await?;
    Ok(request_id)
}

#[derive(Debug, thiserror::Error)]
pub enum ErasureError {
    #[error("erasure request is invalid")]
    InvalidInput,
    #[error("erasure persistence failed")]
    Database(#[from] sqlx::Error),
}

#[cfg(test)]
#[path = "trust_risk.retention.tests.rs"]
mod tests;
