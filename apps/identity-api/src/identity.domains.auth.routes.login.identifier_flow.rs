use uuid::Uuid;

use crate::http::error::AppError;

pub(crate) struct IdentifierChallenge {
    pub principal_id: Option<Uuid>,
    pub next_step: String,
    pub available_methods: Option<Vec<String>>,
}

pub(crate) fn resolve_uniform_identifier_challenge(_email: &str) -> IdentifierChallenge {
    IdentifierChallenge {
        principal_id: None,
        next_step: "pwd".to_string(),
        available_methods: None,
    }
}

pub(crate) async fn resolve_mfa_challenge_methods(
    db: &sqlx::PgPool,
    principal_id: Uuid,
) -> Result<Vec<String>, AppError> {
    crate::domains::auth::mfa::list_login_methods(db, principal_id).await
}

pub(crate) async fn resolve_post_password_challenge(
    db: &sqlx::PgPool,
    principal_id: Uuid,
) -> Result<Option<Vec<String>>, AppError> {
    if !crate::domains::auth::mfa::has_active_factor(db, principal_id).await? {
        return Ok(None);
    }

    Ok(Some(resolve_mfa_challenge_methods(db, principal_id).await?))
}
