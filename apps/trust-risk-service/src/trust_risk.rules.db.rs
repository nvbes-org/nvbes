use chrono::{DateTime, Utc};
use nvbes_trust_risk::rules::RuleSet;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use crate::audit::{self, AuditEvent};

#[derive(Debug, Clone)]
pub struct RuleSetReceipt {
    pub version: String,
    pub state: String,
    pub checksum: String,
    pub updated_at: DateTime<Utc>,
}

pub async fn stage(
    pool: &PgPool,
    version: &str,
    canonical_json: &[u8],
    actor: &str,
    reason: &str,
    audit_days: u32,
) -> Result<RuleSetReceipt, RuleOperationError> {
    validate_operator_input(actor, reason)?;
    let rules = RuleSet::from_json(canonical_json).map_err(|_| RuleOperationError::InvalidRules)?;
    if rules.version() != version {
        return Err(RuleOperationError::InvalidRules);
    }
    let canonical: serde_json::Value =
        serde_json::from_slice(canonical_json).map_err(|_| RuleOperationError::InvalidRules)?;
    let normalized =
        serde_json::to_vec(&canonical).map_err(|_| RuleOperationError::InvalidRules)?;
    let checksum: [u8; 32] = Sha256::digest(&normalized).into();
    let mut tx = pool.begin().await?;
    let updated_at = sqlx::query_scalar::<_, DateTime<Utc>>(
        r#"
        INSERT INTO trust_risk_rule_sets (
            version, feature_version, state, canonical_json, checksum, created_by
        ) VALUES ($1, $2, 'staged', $3, $4, $5)
        RETURNING created_at
        "#,
    )
    .bind(version)
    .bind(rules.feature_version())
    .bind(canonical)
    .bind(checksum.as_slice())
    .bind(actor)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_unique)?;
    audit::append(
        &mut tx,
        AuditEvent {
            action: "rules.stage",
            actor,
            reason,
            target_type: "rule_set",
            target_id: version,
            detail: serde_json::json!({"checksum": hex(&checksum)}),
        },
        audit_days,
    )
    .await?;
    tx.commit().await?;
    Ok(RuleSetReceipt {
        version: version.to_string(),
        state: "staged".to_string(),
        checksum: hex(&checksum),
        updated_at,
    })
}

pub async fn activate(
    pool: &PgPool,
    version: &str,
    actor: &str,
    reason: &str,
    audit_days: u32,
) -> Result<RuleSetReceipt, RuleOperationError> {
    set_active(pool, version, actor, reason, audit_days, "rules.activate").await
}

pub async fn rollback(
    pool: &PgPool,
    version: &str,
    actor: &str,
    reason: &str,
    audit_days: u32,
) -> Result<RuleSetReceipt, RuleOperationError> {
    set_active(pool, version, actor, reason, audit_days, "rules.rollback").await
}

async fn set_active(
    pool: &PgPool,
    version: &str,
    actor: &str,
    reason: &str,
    audit_days: u32,
    action: &str,
) -> Result<RuleSetReceipt, RuleOperationError> {
    validate_operator_input(actor, reason)?;
    let mut tx = pool.begin().await?;
    let row = sqlx::query_as::<_, (String, String, Vec<u8>)>(
        "SELECT state, created_by, checksum FROM trust_risk_rule_sets WHERE version = $1 FOR UPDATE",
    ).bind(version).fetch_optional(&mut *tx).await?.ok_or(RuleOperationError::NotFound)?;
    if row.0 != "staged" && row.0 != "retired" {
        return Err(RuleOperationError::InvalidState);
    }
    if row.1 == actor {
        return Err(RuleOperationError::DualControl);
    }
    sqlx::query("UPDATE trust_risk_rule_sets SET state = 'retired' WHERE state = 'active'")
        .execute(&mut *tx)
        .await?;
    let updated_at = sqlx::query_scalar::<_, DateTime<Utc>>(
        "UPDATE trust_risk_rule_sets SET state = 'active', activated_by = $2, activated_at = clock_timestamp() WHERE version = $1 RETURNING activated_at",
    ).bind(version).bind(actor).fetch_one(&mut *tx).await?;
    audit::append(
        &mut tx,
        AuditEvent {
            action,
            actor,
            reason,
            target_type: "rule_set",
            target_id: version,
            detail: serde_json::json!({}),
        },
        audit_days,
    )
    .await?;
    tx.commit().await?;
    Ok(RuleSetReceipt {
        version: version.to_string(),
        state: "active".to_string(),
        checksum: hex(&row.2),
        updated_at,
    })
}

fn validate_operator_input(actor: &str, reason: &str) -> Result<(), RuleOperationError> {
    if actor.len() < 3 || reason.trim().len() < 3 || reason.len() > 300 {
        Err(RuleOperationError::InvalidInput)
    } else {
        Ok(())
    }
}

fn map_unique(error: sqlx::Error) -> RuleOperationError {
    if error
        .as_database_error()
        .is_some_and(|error| error.is_unique_violation())
    {
        RuleOperationError::Conflict
    } else {
        RuleOperationError::Database(error)
    }
}

fn hex(value: &[u8]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Debug, thiserror::Error)]
pub enum RuleOperationError {
    #[error("rule request is invalid")]
    InvalidInput,
    #[error("rule set is invalid")]
    InvalidRules,
    #[error("rule set already exists")]
    Conflict,
    #[error("rule set was not found")]
    NotFound,
    #[error("rule set state is invalid")]
    InvalidState,
    #[error("rule activation requires a different actor")]
    DualControl,
    #[error("rule persistence failed")]
    Database(#[from] sqlx::Error),
}

#[cfg(test)]
#[path = "trust_risk.rules.tests.rs"]
mod tests;
