use chrono::{DateTime, Duration, Utc};
use nvbes_trust_risk::label::LabelAssertion;
#[cfg(test)]
use nvbes_trust_risk::proto::nvbes::trust_risk::v1 as pb;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::constant_time_eq;

#[derive(Debug, Clone)]
pub struct LabelReceipt {
    pub id: Uuid,
    pub accepted_at: DateTime<Utc>,
    pub duplicate: bool,
}

pub async fn persist_label(
    pool: &PgPool,
    label: &LabelAssertion,
    retention_days: u32,
) -> Result<LabelReceipt, LabelPersistenceError> {
    let fingerprint = fingerprint(label);
    let mut tx = pool.begin().await?;
    if let Some(corrected) = label.corrects_label_id() {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM trust_risk_labels WHERE id = $1)",
        )
        .bind(corrected)
        .fetch_one(&mut *tx)
        .await?;
        if !exists {
            return Err(LabelPersistenceError::InvalidCorrection);
        }
    }
    let expires_at = label.knowledge_at() + Duration::days(i64::from(retention_days));
    let inserted = sqlx::query_scalar::<_, DateTime<Utc>>(
        r#"
        INSERT INTO trust_risk_labels (
            id, schema_version, producer, evaluation_id, review_case_id, kind,
            source_class, source_id, confidence, actor, knowledge_at,
            evidence_reference, mapping_version, corrects_label_id, fingerprint, expires_at
        ) VALUES ($1, 1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        ON CONFLICT (id) DO NOTHING RETURNING accepted_at
        "#,
    )
    .bind(label.id())
    .bind(label.producer())
    .bind(label.evaluation_id())
    .bind(label.review_case_id())
    .bind(label.kind() as i16)
    .bind(label.source_class() as i16)
    .bind(label.source_id())
    .bind(label.confidence())
    .bind(label.actor())
    .bind(label.knowledge_at())
    .bind(label.evidence_reference())
    .bind(label.mapping_version())
    .bind(label.corrects_label_id())
    .bind(fingerprint.as_slice())
    .bind(expires_at)
    .fetch_optional(&mut *tx)
    .await?;
    let (accepted_at, duplicate) = if let Some(accepted_at) = inserted {
        recompute_canonical(&mut tx, label.evaluation_id()).await?;
        (accepted_at, false)
    } else {
        let row = sqlx::query_as::<_, (Vec<u8>, DateTime<Utc>)>(
            "SELECT fingerprint, accepted_at FROM trust_risk_labels WHERE id = $1",
        )
        .bind(label.id())
        .fetch_one(&mut *tx)
        .await?;
        if !constant_time_eq(&row.0, &fingerprint) {
            return Err(LabelPersistenceError::Conflict);
        }
        (row.1, true)
    };
    tx.commit().await?;
    Ok(LabelReceipt {
        id: label.id(),
        accepted_at,
        duplicate,
    })
}

async fn recompute_canonical(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    evaluation_id: Uuid,
) -> Result<(), sqlx::Error> {
    let selected = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT label.id FROM trust_risk_labels AS label
        WHERE label.evaluation_id = $1 AND label.source_class IN (1, 2, 3)
          AND NOT EXISTS (SELECT 1 FROM trust_risk_labels AS correction WHERE correction.corrects_label_id = label.id)
        ORDER BY CASE label.source_class WHEN 1 THEN 3 WHEN 2 THEN 2 WHEN 3 THEN 1 ELSE 0 END DESC,
                 label.confidence DESC, label.knowledge_at DESC, label.id
        LIMIT 1
        "#,
    ).bind(evaluation_id).fetch_optional(&mut **tx).await?;
    if let Some(label_id) = selected {
        sqlx::query(
            "INSERT INTO trust_risk_canonical_labels (evaluation_id, label_id) VALUES ($1, $2) ON CONFLICT (evaluation_id) DO UPDATE SET label_id = EXCLUDED.label_id, resolved_at = clock_timestamp()",
        ).bind(evaluation_id).bind(label_id).execute(&mut **tx).await?;
    }
    Ok(())
}

#[cfg(test)]
pub fn authority_rank(source: pb::LabelSourceClass) -> Option<u8> {
    match source {
        pb::LabelSourceClass::Human => Some(3),
        pb::LabelSourceClass::AuthoritativeExternal => Some(2),
        pb::LabelSourceClass::VerifiedProduct => Some(1),
        pb::LabelSourceClass::Heuristic | pb::LabelSourceClass::Unspecified => None,
    }
}

pub fn fingerprint(label: &LabelAssertion) -> [u8; 32] {
    let mut hash = Sha256::new();
    for bytes in [label.id().as_bytes(), label.evaluation_id().as_bytes()] {
        hash.update(bytes);
    }
    for value in [label.producer(), label.source_id(), label.mapping_version()] {
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value.as_bytes());
    }
    hash.update((label.kind() as i32).to_be_bytes());
    hash.update((label.source_class() as i32).to_be_bytes());
    hash.update(label.confidence().to_bits().to_be_bytes());
    hash.update(label.knowledge_at().to_rfc3339().as_bytes());
    for value in [label.actor(), label.evidence_reference()] {
        if let Some(value) = value {
            hash.update((value.len() as u64).to_be_bytes());
            hash.update(value.as_bytes());
        } else {
            hash.update(0_u64.to_be_bytes());
        }
    }
    if let Some(id) = label.review_case_id() {
        hash.update(id.as_bytes());
    }
    if let Some(id) = label.corrects_label_id() {
        hash.update(id.as_bytes());
    }
    hash.finalize().into()
}

#[derive(Debug, thiserror::Error)]
pub enum LabelPersistenceError {
    #[error("label identifier conflicts with different content")]
    Conflict,
    #[error("label correction target does not exist")]
    InvalidCorrection,
    #[error("label persistence failed")]
    Database(#[from] sqlx::Error),
}

#[cfg(test)]
#[path = "trust_risk.labels.tests.rs"]
mod tests;
