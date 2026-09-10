use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// Serializes factor management on the principal before locking the session.
pub(crate) async fn owner(
    tx: &mut Transaction<'_, Postgres>,
    token: &str,
    strong: bool,
) -> Result<Option<Uuid>, sqlx::Error> {
    Ok(crate::authentication_session::load(tx, token)
        .await?
        .filter(|session| !strong || session.recent_strong)
        .map(|session| session.principal))
}
