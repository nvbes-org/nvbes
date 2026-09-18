use std::future::Future;

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{RecoveryNotification, authenticate, create_synthetic_identity, hash_token};
use crate::recovery::{request_recovery, reset_password, validate_password_pair};

#[derive(Debug, Serialize)]
pub struct SyntheticSmokeResult {
    pub principal_id: Uuid,
    pub session_rotated: bool,
    pub recovery_consumed: bool,
    pub audit_events: i64,
}

pub async fn run(
    db: &PgPool,
    email: &str,
    initial_password: &str,
    recovered_password: &str,
) -> anyhow::Result<SyntheticSmokeResult> {
    run_with_delivery(db, email, initial_password, recovered_password, |_| async {
        Ok(())
    })
    .await
}

pub async fn run_with_delivery<F, Fut>(
    db: &PgPool,
    email: &str,
    initial_password: &str,
    recovered_password: &str,
    deliver_recovery: F,
) -> anyhow::Result<SyntheticSmokeResult>
where
    F: FnOnce(RecoveryNotification) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    validate_password_pair(initial_password, recovered_password)?;
    let principal_id = create_synthetic_identity(db, email, initial_password).await?;
    let first_session = authenticate(db, email, initial_password).await?;
    let recovery = request_recovery(db, email).await?;
    deliver_recovery(recovery.clone()).await?;
    reset_password(db, &recovery.token, recovered_password).await?;
    let second_session = authenticate(db, email, recovered_password).await?;
    let old_session_active: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM identity_sessions WHERE token_hash = $1 AND revoked_at IS NULL)",
    )
    .bind(hash_token(&first_session))
    .fetch_one(db)
    .await?;
    let recovery_consumed: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM identity_recovery_challenges WHERE token_hash = $1 AND consumed_at IS NOT NULL)",
    )
    .bind(hash_token(&recovery.token))
    .fetch_one(db)
    .await?;
    let audit_events: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM identity_audit_events WHERE principal_id = $1")
            .bind(principal_id)
            .fetch_one(db)
            .await?;

    Ok(SyntheticSmokeResult {
        principal_id,
        session_rotated: !old_session_active && first_session != second_session,
        recovery_consumed,
        audit_events,
    })
}
