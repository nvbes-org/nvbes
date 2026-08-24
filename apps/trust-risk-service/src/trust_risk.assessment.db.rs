use chrono::{DateTime, Duration, Utc};
use nvbes_trust_risk::{
    assessment::Assessment,
    proto::nvbes::trust_risk::v1 as pb,
    rules::{FeatureMap, RuleSet, evaluate},
    types::{Recommendation, RiskBand},
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    auth::constant_time_eq,
    ingress_db::{
        PersistSignalError, fingerprint as signal_fingerprint, persist_signal_in_transaction,
    },
    projection::derive_features,
};

#[derive(Debug, Clone, PartialEq)]
pub struct StoredEvaluation {
    pub id: Uuid,
    pub score: u8,
    pub band: RiskBand,
    pub recommendation: Recommendation,
    pub reasons: Vec<String>,
    pub feature_version: String,
    pub rule_set_version: String,
    pub evaluated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub duplicate: bool,
}

pub async fn assess(
    pool: &PgPool,
    wire: &pb::AssessRiskRequest,
    assessment: &Assessment,
    evaluation_retention_days: u32,
    signal_retention_days: u32,
) -> Result<StoredEvaluation, AssessmentPersistenceError> {
    let request_fingerprint = fingerprint(assessment);
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("{}:{}", assessment.producer(), assessment.key()))
        .execute(&mut *tx)
        .await?;
    if let Some(existing) = existing(&mut tx, assessment, &request_fingerprint).await? {
        tx.commit().await?;
        return Ok(existing);
    }

    for (wire_signal, signal) in wire
        .instantaneous_signals
        .iter()
        .zip(assessment.instantaneous_signals())
    {
        persist_signal_in_transaction(&mut tx, wire_signal, signal, signal_retention_days).await?;
    }
    let evaluated_at = crate::database::database_now(pool).await?;
    let mut features = load_features(&mut tx, assessment).await?;
    combine_instantaneous(
        &mut features,
        derive_features(assessment.instantaneous_signals(), evaluated_at),
    );
    let (rule_version, feature_version, rule_json) = sqlx::query_as::<_, (String, String, serde_json::Value)>(
        "SELECT version, feature_version, canonical_json FROM trust_risk_rule_sets WHERE state = 'active'",
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AssessmentPersistenceError::NoActiveRules)?;
    let rule_bytes =
        serde_json::to_vec(&rule_json).map_err(|_| AssessmentPersistenceError::InvalidRules)?;
    let rules =
        RuleSet::from_json(&rule_bytes).map_err(|_| AssessmentPersistenceError::InvalidRules)?;
    if rules.feature_version() != feature_version {
        return Err(AssessmentPersistenceError::InvalidRules);
    }
    let result = evaluate(&rules, &features);
    let id = Uuid::new_v4();
    let expires_at = evaluated_at + Duration::days(i64::from(evaluation_retention_days));
    sqlx::query(
        r#"
        INSERT INTO trust_risk_evaluations (
            id, producer, assessment_key, operation_class, request_fingerprint,
            score, band, recommendation, feature_version, rule_set_version,
            evaluated_at, expires_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
    )
    .bind(id)
    .bind(assessment.producer())
    .bind(assessment.key())
    .bind(assessment.operation_class())
    .bind(request_fingerprint.as_slice())
    .bind(i16::from(result.score))
    .bind(band_name(result.band))
    .bind(recommendation_name(result.recommendation))
    .bind(&feature_version)
    .bind(&rule_version)
    .bind(evaluated_at)
    .bind(expires_at)
    .execute(&mut *tx)
    .await?;
    persist_snapshot(&mut tx, id, &features, &result.reason_codes).await?;
    tx.commit().await?;

    Ok(StoredEvaluation {
        id,
        score: result.score,
        band: result.band,
        recommendation: result.recommendation,
        reasons: result.reason_codes,
        feature_version,
        rule_set_version: rule_version,
        evaluated_at,
        expires_at,
        duplicate: false,
    })
}

async fn existing(
    tx: &mut Transaction<'_, Postgres>,
    assessment: &Assessment,
    fingerprint: &[u8; 32],
) -> Result<Option<StoredEvaluation>, AssessmentPersistenceError> {
    type Row = (
        Uuid,
        Vec<u8>,
        i16,
        String,
        String,
        String,
        String,
        DateTime<Utc>,
        DateTime<Utc>,
    );
    let row = sqlx::query_as::<_, Row>(
        r#"
        SELECT id, request_fingerprint, score, band, recommendation, feature_version,
               rule_set_version, evaluated_at, expires_at
        FROM trust_risk_evaluations WHERE producer = $1 AND assessment_key = $2
        "#,
    )
    .bind(assessment.producer())
    .bind(assessment.key())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    if !constant_time_eq(&row.1, fingerprint) {
        return Err(AssessmentPersistenceError::Conflict);
    }
    let reasons = sqlx::query_scalar::<_, String>(
        "SELECT code FROM trust_risk_evaluation_reasons WHERE evaluation_id = $1 ORDER BY ordinal",
    )
    .bind(row.0)
    .fetch_all(&mut **tx)
    .await?;
    Ok(Some(StoredEvaluation {
        id: row.0,
        score: u8::try_from(row.2).map_err(|_| AssessmentPersistenceError::CorruptLedger)?,
        band: parse_band(&row.3)?,
        recommendation: parse_recommendation(&row.4)?,
        reasons,
        feature_version: row.5,
        rule_set_version: row.6,
        evaluated_at: row.7,
        expires_at: row.8,
        duplicate: true,
    }))
}

async fn load_features(
    tx: &mut Transaction<'_, Postgres>,
    assessment: &Assessment,
) -> Result<FeatureMap, AssessmentPersistenceError> {
    let mut combined = FeatureMap::new();
    for subject in assessment.subjects() {
        let value = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT features FROM trust_risk_feature_state WHERE subject_kind = $1 AND namespace = $2 AND opaque_id = $3 AND feature_version = 'features-v1'",
        )
        .bind(subject.kind() as i16)
        .bind(subject.namespace())
        .bind(subject.opaque_id())
        .fetch_optional(&mut **tx)
        .await?;
        if let Some(value) = value {
            let features: FeatureMap = serde_json::from_value(value)
                .map_err(|_| AssessmentPersistenceError::CorruptFeatureState)?;
            for (name, value) in features {
                let current = combined.entry(name).or_default();
                *current = current.max(value);
            }
        }
    }
    Ok(combined)
}

fn combine_instantaneous(features: &mut FeatureMap, instantaneous: FeatureMap) {
    for (name, value) in instantaneous {
        let current = features.entry(name.clone()).or_default();
        if name.ends_with("_max_1h") {
            *current = current.max(value);
        } else if !matches!(name.as_str(), "negative_labels" | "positive_labels") {
            *current += value;
        }
    }
}

async fn persist_snapshot(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    features: &FeatureMap,
    reasons: &[String],
) -> Result<(), sqlx::Error> {
    for (name, value) in features {
        sqlx::query("INSERT INTO trust_risk_evaluation_features (evaluation_id, name, value) VALUES ($1, $2, $3)")
            .bind(id).bind(name).bind(value).execute(&mut **tx).await?;
    }
    for (ordinal, code) in reasons.iter().enumerate() {
        sqlx::query("INSERT INTO trust_risk_evaluation_reasons (evaluation_id, ordinal, code) VALUES ($1, $2, $3)")
            .bind(id).bind(ordinal as i16).bind(code).execute(&mut **tx).await?;
    }
    Ok(())
}

pub fn fingerprint(assessment: &Assessment) -> [u8; 32] {
    let mut hash = Sha256::new();
    for value in [
        assessment.producer(),
        assessment.key(),
        assessment.operation_class(),
    ] {
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value.as_bytes());
    }
    for subject in assessment.subjects() {
        hash.update((subject.kind() as i32).to_be_bytes());
        hash.update(subject.namespace().as_bytes());
        hash.update(subject.opaque_id().as_bytes());
    }
    for signal in assessment.instantaneous_signals() {
        hash.update(signal_fingerprint(signal));
    }
    hash.finalize().into()
}

fn band_name(value: RiskBand) -> &'static str {
    match value {
        RiskBand::Low => "low",
        RiskBand::Elevated => "elevated",
        RiskBand::High => "high",
        RiskBand::Critical => "critical",
    }
}
fn recommendation_name(value: Recommendation) -> &'static str {
    match value {
        Recommendation::Allow => "allow",
        Recommendation::Challenge => "challenge",
        Recommendation::Review => "review",
        Recommendation::Deny => "deny",
    }
}
fn parse_band(value: &str) -> Result<RiskBand, AssessmentPersistenceError> {
    match value {
        "low" => Ok(RiskBand::Low),
        "elevated" => Ok(RiskBand::Elevated),
        "high" => Ok(RiskBand::High),
        "critical" => Ok(RiskBand::Critical),
        _ => Err(AssessmentPersistenceError::CorruptLedger),
    }
}
fn parse_recommendation(value: &str) -> Result<Recommendation, AssessmentPersistenceError> {
    match value {
        "allow" => Ok(Recommendation::Allow),
        "challenge" => Ok(Recommendation::Challenge),
        "review" => Ok(Recommendation::Review),
        "deny" => Ok(Recommendation::Deny),
        _ => Err(AssessmentPersistenceError::CorruptLedger),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AssessmentPersistenceError {
    #[error("assessment key conflicts with different content")]
    Conflict,
    #[error("no valid active rule set is available")]
    NoActiveRules,
    #[error("active rule set is invalid")]
    InvalidRules,
    #[error("feature state is invalid")]
    CorruptFeatureState,
    #[error("evaluation ledger is invalid")]
    CorruptLedger,
    #[error(transparent)]
    Signal(#[from] PersistSignalError),
    #[error("assessment database operation failed")]
    Database(#[from] sqlx::Error),
}

#[cfg(test)]
#[path = "trust_risk.assessment.tests.rs"]
mod tests;
