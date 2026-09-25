use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::sync::watch;
use uuid::Uuid;

use crate::{
    app::TrustRiskState,
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
    if !erasure_input_is_valid(kind, namespace, opaque_id, actor, reason) {
        return Err(ErasureError::InvalidInput);
    }
    let request_id = Uuid::new_v4();
    let audit_target = request_id.to_string();
    let erased_reference = Sha256::digest(format!("{namespace}:{opaque_id}").as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let mut tx = pool.begin().await?;
    let held = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM trust_risk_signals AS signal
            JOIN trust_risk_signal_subjects AS subject ON subject.signal_id = signal.id
            WHERE subject.kind = $1 AND subject.namespace = $2 AND subject.opaque_id = $3
              AND signal.legal_hold
        )
        "#,
    )
    .bind(kind)
    .bind(namespace)
    .bind(opaque_id)
    .fetch_one(&mut *tx)
    .await?;
    if held {
        return Err(ErasureError::LegalHold);
    }
    sqlx::query(
        r#"
        DELETE FROM trust_risk_signals AS signal
        WHERE EXISTS (
            SELECT 1 FROM trust_risk_signal_subjects AS subject
            WHERE subject.signal_id = signal.id
              AND subject.kind = $1 AND subject.namespace = $2 AND subject.opaque_id = $3
        )
        "#,
    )
    .bind(kind)
    .bind(namespace)
    .bind(opaque_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM trust_risk_signal_subjects WHERE kind = $1 AND namespace = $2 AND opaque_id = $3")
        .bind(kind).bind(namespace).bind(opaque_id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM trust_risk_feature_state WHERE subject_kind = $1 AND namespace = $2 AND opaque_id = $3")
        .bind(kind).bind(namespace).bind(opaque_id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO trust_risk_rebuild_requests (id, subject_kind, namespace, opaque_id, reason, requested_by) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(request_id).bind(kind).bind("erased.sha256").bind(erased_reference).bind(reason.trim()).bind(actor)
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
    #[error("subject data is protected by an active legal hold")]
    LegalHold,
    #[error("erasure persistence failed")]
    Database(#[from] sqlx::Error),
}

pub(crate) fn erasure_input_is_valid(
    kind: i16,
    namespace: &str,
    opaque_id: &str,
    actor: &str,
    reason: &str,
) -> bool {
    (1..=6).contains(&kind)
        && namespace.len() >= 3
        && opaque_id.len() >= 8
        && actor.len() >= 3
        && reason.trim().len() >= 3
}

pub async fn run(state: TrustRiskState, mut shutdown: watch::Receiver<bool>) {
    loop {
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(3600)) => {
                match apply(&state.db).await {
                    Ok(result) => tracing::info!(signals = result.signals, features = result.features, evaluations = result.evaluations, labels = result.labels, reviews = result.reviews, audit = result.audit, "trust/risk retention applied"),
                    Err(error) => {
                        crate::error_reporting::capture_operation(
                            &state.config,
                            "retention.apply",
                            &error,
                        );
                        tracing::warn!(error = %error, "trust/risk retention failed");
                    },
                }
            }
            result = shutdown.changed() => if result.is_err() || *shutdown.borrow() { break; },
        }
    }
}

#[cfg(test)]
#[path = "trust_risk.retention.tests.rs"]
mod tests;
