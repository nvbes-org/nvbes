use chrono::{DateTime, Duration, Utc};
use nvbes_trust_risk::{
    attribute::Attribute,
    proto::nvbes::trust_risk::v1 as pb,
    signal::{RiskSignal, SubjectReference},
};
use prost::Message;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::constant_time_eq;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalReceipt {
    pub id: Uuid,
    pub accepted_at: DateTime<Utc>,
    pub duplicate: bool,
}

pub async fn persist_signal(
    pool: &PgPool,
    wire: &pb::RiskSignal,
    signal: &RiskSignal,
    retention_days: u32,
) -> Result<SignalReceipt, PersistSignalError> {
    let payload = wire.encode_to_vec();
    if payload.len() > 192 * 1024 {
        return Err(PersistSignalError::PayloadTooLarge);
    }
    let mut tx = pool.begin().await?;
    let receipt = persist_signal_in_transaction(&mut tx, wire, signal, retention_days).await?;
    tx.commit().await?;
    Ok(receipt)
}

pub async fn persist_signal_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    wire: &pb::RiskSignal,
    signal: &RiskSignal,
    retention_days: u32,
) -> Result<SignalReceipt, PersistSignalError> {
    let fingerprint = fingerprint(signal);
    let payload = wire.encode_to_vec();
    if payload.len() > 192 * 1024 {
        return Err(PersistSignalError::PayloadTooLarge);
    }
    let retention_deadline = signal.occurred_at() + Duration::days(i64::from(retention_days));
    let inserted = sqlx::query_scalar::<_, DateTime<Utc>>(
        r#"
        INSERT INTO trust_risk_signals (
            id, schema_version, producer, signal_kind, occurred_at, partition_key,
            scope, fingerprint, payload, retention_deadline
        ) VALUES ($1, 1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (id) DO NOTHING
        RETURNING accepted_at
        "#,
    )
    .bind(signal.id())
    .bind(signal.producer())
    .bind(signal.kind())
    .bind(signal.occurred_at())
    .bind(signal.partition_key())
    .bind(signal.scope() as i16)
    .bind(fingerprint.as_slice())
    .bind(payload)
    .bind(retention_deadline)
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(accepted_at) = inserted {
        insert_subjects(tx, signal.id(), signal.subjects()).await?;
        sqlx::query(
            r#"
            INSERT INTO trust_risk_outbox (
                aggregate_id, event_kind, partition_key, occurred_at, payload
            ) VALUES ($1, 'signal.accepted', $2, $3, jsonb_build_object(
                'signal_id', $1::text, 'producer', $4::text, 'signal_kind', $5::text
            ))
            "#,
        )
        .bind(signal.id())
        .bind(signal.partition_key())
        .bind(signal.occurred_at())
        .bind(signal.producer())
        .bind(signal.kind())
        .execute(&mut **tx)
        .await?;
        return Ok(SignalReceipt {
            id: signal.id(),
            accepted_at,
            duplicate: false,
        });
    }

    let existing = sqlx::query_as::<_, (Vec<u8>, DateTime<Utc>)>(
        "SELECT fingerprint, accepted_at FROM trust_risk_signals WHERE id = $1",
    )
    .bind(signal.id())
    .fetch_one(&mut **tx)
    .await?;
    if !constant_time_eq(&existing.0, &fingerprint) {
        return Err(PersistSignalError::Conflict);
    }
    Ok(SignalReceipt {
        id: signal.id(),
        accepted_at: existing.1,
        duplicate: true,
    })
}

async fn insert_subjects(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    signal_id: Uuid,
    subjects: &[SubjectReference],
) -> Result<(), sqlx::Error> {
    for (ordinal, subject) in subjects.iter().enumerate() {
        sqlx::query(
            r#"
            INSERT INTO trust_risk_signal_subjects (
                signal_id, ordinal, kind, namespace, opaque_id, scope, tenant_id
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(signal_id)
        .bind(ordinal as i16)
        .bind(subject.kind() as i16)
        .bind(subject.namespace())
        .bind(subject.opaque_id())
        .bind(subject.scope() as i16)
        .bind(subject.tenant_id())
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

pub fn fingerprint(signal: &RiskSignal) -> [u8; 32] {
    let mut hash = Sha256::new();
    field(&mut hash, signal.id().as_bytes());
    field(&mut hash, signal.producer().as_bytes());
    field(&mut hash, signal.kind().as_bytes());
    field(&mut hash, signal.occurred_at().to_rfc3339().as_bytes());
    field(&mut hash, signal.partition_key().as_bytes());
    field(&mut hash, &(signal.scope() as i32).to_be_bytes());
    for subject in signal.subjects() {
        field(&mut hash, &(subject.kind() as i32).to_be_bytes());
        field(&mut hash, subject.namespace().as_bytes());
        field(&mut hash, subject.opaque_id().as_bytes());
        field(&mut hash, &(subject.scope() as i32).to_be_bytes());
        match subject.tenant_id() {
            Some(tenant_id) => field(&mut hash, tenant_id.as_bytes()),
            None => field(&mut hash, &[]),
        }
    }
    for (name, value) in signal.attributes() {
        field(&mut hash, name.as_bytes());
        attribute(&mut hash, value);
    }
    hash.finalize().into()
}

fn attribute(hash: &mut Sha256, value: &Attribute) {
    match value {
        Attribute::String(value) => {
            field(hash, b"s");
            field(hash, value.as_bytes());
        }
        Attribute::Signed(value) => {
            field(hash, b"i");
            field(hash, &value.to_be_bytes());
        }
        Attribute::Unsigned(value) => {
            field(hash, b"u");
            field(hash, &value.to_be_bytes());
        }
        Attribute::Bool(value) => {
            field(hash, b"b");
            field(hash, &[*value as u8]);
        }
        Attribute::Decimal(value) => {
            field(hash, b"d");
            field(hash, &value.to_bits().to_be_bytes());
        }
        Attribute::Opaque(value) => {
            field(hash, b"o");
            field(hash, value);
        }
    }
}

fn field(hash: &mut Sha256, value: &[u8]) {
    hash.update((value.len() as u64).to_be_bytes());
    hash.update(value);
}

#[derive(Debug, thiserror::Error)]
pub enum PersistSignalError {
    #[error("signal identifier conflicts with different content")]
    Conflict,
    #[error("signal payload exceeds the persistence budget")]
    PayloadTooLarge,
    #[error("signal persistence failed")]
    Database(#[from] sqlx::Error),
}

#[cfg(test)]
#[path = "trust_risk.ingress.tests.rs"]
mod tests;
