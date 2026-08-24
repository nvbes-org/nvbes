use nvbes_trust_risk::{proto::nvbes::trust_risk::v1 as pb, signal::RiskSignal};
use prost::Message;
use sqlx::PgPool;
use uuid::Uuid;

use crate::projection::{FEATURE_VERSION, derive_features, is_too_late, retry_delay};

#[derive(Debug)]
struct ClaimedSignal {
    id: Uuid,
    payload: Vec<u8>,
    attempts: i32,
}

pub async fn process_batch(
    pool: &PgPool,
    worker_id: Uuid,
    limit: i64,
) -> Result<usize, ProjectionError> {
    let claims = claim(pool, worker_id, limit.clamp(1, 256)).await?;
    let count = claims.len();
    for claim in claims {
        if let Err(error) = recompute(pool, &claim).await {
            record_failure(pool, claim.id, claim.attempts as u32, error.code()).await?;
        }
    }
    Ok(count)
}

async fn claim(
    pool: &PgPool,
    worker_id: Uuid,
    limit: i64,
) -> Result<Vec<ClaimedSignal>, sqlx::Error> {
    sqlx::query_as::<_, (Uuid, Vec<u8>, i32)>(
        r#"
        WITH candidates AS (
            SELECT id
            FROM trust_risk_signals
            WHERE projected_at IS NULL
              AND quarantined_at IS NULL
              AND next_projection_at <= clock_timestamp()
              AND (lease_expires_at IS NULL OR lease_expires_at <= clock_timestamp())
            ORDER BY next_projection_at, accepted_at, id
            FOR UPDATE SKIP LOCKED
            LIMIT $1
        )
        UPDATE trust_risk_signals AS signal
        SET lease_owner = $2,
            lease_expires_at = clock_timestamp() + INTERVAL '30 seconds',
            projection_attempts = projection_attempts + 1
        FROM candidates
        WHERE signal.id = candidates.id
        RETURNING signal.id, signal.payload, signal.projection_attempts
        "#,
    )
    .bind(limit)
    .bind(worker_id)
    .fetch_all(pool)
    .await
    .map(|rows| {
        rows.into_iter()
            .map(|(id, payload, attempts)| ClaimedSignal {
                id,
                payload,
                attempts,
            })
            .collect()
    })
}

async fn recompute(pool: &PgPool, claim: &ClaimedSignal) -> Result<(), ProjectionError> {
    let source = decode(&claim.payload)?;
    let watermark = crate::database::database_now(pool).await?;
    if is_too_late(source.occurred_at(), watermark) {
        crate::risk_metrics::projection("too_late", 1);
    }
    let mut tx = pool.begin().await?;
    for subject in source.subjects() {
        let payloads = sqlx::query_scalar::<_, Vec<u8>>(
            r#"
            SELECT signal.payload
            FROM trust_risk_signals AS signal
            JOIN trust_risk_signal_subjects AS subject ON subject.signal_id = signal.id
            WHERE subject.kind = $1 AND subject.namespace = $2 AND subject.opaque_id = $3
              AND signal.occurred_at > $4 - INTERVAL '24 hours'
              AND signal.occurred_at <= $4
              AND signal.quarantined_at IS NULL
            ORDER BY signal.occurred_at, signal.id
            "#,
        )
        .bind(subject.kind() as i16)
        .bind(subject.namespace())
        .bind(subject.opaque_id())
        .bind(watermark)
        .fetch_all(&mut *tx)
        .await?;
        let signals = payloads
            .iter()
            .map(|payload| decode(payload))
            .collect::<Result<Vec<_>, _>>()?;
        let features = derive_features(&signals, watermark);
        sqlx::query(
            r#"
            INSERT INTO trust_risk_feature_state (
                subject_kind, namespace, opaque_id, feature_version, features,
                event_watermark, expires_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $6 + INTERVAL '25 hours')
            ON CONFLICT (subject_kind, namespace, opaque_id, feature_version)
            DO UPDATE SET features = EXCLUDED.features,
                event_watermark = EXCLUDED.event_watermark,
                rebuilt_at = clock_timestamp(), expires_at = EXCLUDED.expires_at
            "#,
        )
        .bind(subject.kind() as i16)
        .bind(subject.namespace())
        .bind(subject.opaque_id())
        .bind(FEATURE_VERSION)
        .bind(serde_json::to_value(features).map_err(|_| ProjectionError::InvalidPayload)?)
        .bind(watermark)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        "UPDATE trust_risk_signals SET projected_at = clock_timestamp(), lease_owner = NULL, lease_expires_at = NULL WHERE id = $1",
    )
    .bind(claim.id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn record_failure(
    pool: &PgPool,
    signal_id: Uuid,
    attempts: u32,
    code: &str,
) -> Result<(), sqlx::Error> {
    if let Some(delay) = retry_delay(attempts, signal_id) {
        sqlx::query(
            "UPDATE trust_risk_signals SET lease_owner = NULL, lease_expires_at = NULL, next_projection_at = clock_timestamp() + ($2 * INTERVAL '1 second') WHERE id = $1",
        )
        .bind(signal_id)
        .bind(delay.as_secs() as i64)
        .execute(pool)
        .await?;
    } else {
        let mut tx = pool.begin().await?;
        sqlx::query(
            "UPDATE trust_risk_signals SET lease_owner = NULL, lease_expires_at = NULL, quarantined_at = clock_timestamp() WHERE id = $1",
        )
        .bind(signal_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO trust_risk_projection_quarantine (signal_id, error_code, attempts) VALUES ($1, $2, $3) ON CONFLICT (signal_id) DO UPDATE SET error_code = EXCLUDED.error_code, attempts = EXCLUDED.attempts, quarantined_at = clock_timestamp()",
        )
        .bind(signal_id)
        .bind(code)
        .bind(attempts as i32)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
    }
    Ok(())
}

fn decode(payload: &[u8]) -> Result<RiskSignal, ProjectionError> {
    let wire = pb::RiskSignal::decode(payload).map_err(|_| ProjectionError::InvalidPayload)?;
    RiskSignal::try_from(wire).map_err(|_| ProjectionError::InvalidPayload)
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectionError {
    #[error("projection source payload is invalid")]
    InvalidPayload,
    #[error("projection database operation failed")]
    Database(#[from] sqlx::Error),
}

impl ProjectionError {
    fn code(&self) -> &'static str {
        match self {
            Self::InvalidPayload => "invalid_payload",
            Self::Database(_) => "database",
        }
    }
}
