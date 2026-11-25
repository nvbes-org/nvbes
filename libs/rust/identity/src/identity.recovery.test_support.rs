use crate::{
    auth::{self, RecoveryNotification},
    test_fixtures::isolated_database,
};
use sqlx::PgPool;
use uuid::Uuid;

pub(crate) const INITIAL: &str = "Initial-recovery-database-password!";

/// An isolated schema with one verified human principal holding a password.
pub(crate) async fn fixture() -> (PgPool, Uuid, String) {
    let db = isolated_database().await;
    let email = format!("recovery-{}@example.invalid", Uuid::new_v4());
    let principal = auth::create_synthetic_identity(&db, &email, INITIAL)
        .await
        .unwrap();
    (db, principal, email)
}

/// Issues a recovery challenge directly, bypassing the encrypted delivery the
/// HTTP flow uses, so tests can read the one-time token.
pub(crate) async fn request_recovery(
    db: &PgPool,
    email: &str,
) -> anyhow::Result<RecoveryNotification> {
    let mut tx = db.begin().await?;
    let recovery = crate::recovery::issue(&mut tx, email).await?;
    tx.commit().await?;
    Ok(recovery)
}
