use axum::http::HeaderMap;
use uuid::Uuid;

use crate::domains::auth::types::AuthContext;
use crate::http::error::AppError;

pub(crate) struct AuthorizationSubject {
    pub auth: AuthContext,
    pub workspace_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
}

pub(crate) async fn authenticate_authorization_subject(
    db: &sqlx::PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &crate::domains::auth::jwt::JwtService,
    headers: &HeaderMap,
    authuser: Option<&str>,
    client_id: &str,
) -> Result<AuthorizationSubject, AppError> {
    let authuser = authuser.unwrap_or("0");
    let auth = if let Some(token) = crate::http::request::authorization_bearer_token(headers)? {
        crate::domains::auth::sessions::authenticate(db, redis, jwt, &token).await?
    } else {
        let cookie = crate::http::request::browser_session_token_with_authuser(headers, authuser)?;
        crate::domains::auth::sessions::authenticate_browser_session(db, redis, &cookie, headers)
            .await?
    };

    ensure_required_context(client_id, auth.tenant_id, auth.workspace_id)?;

    Ok(AuthorizationSubject {
        workspace_id: auth.workspace_id,
        tenant_id: auth.tenant_id,
        auth,
    })
}

fn ensure_required_context(
    client_id: &str,
    tenant_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
) -> Result<(), AppError> {
    if tenant_id.is_none() {
        return Err(AppError::forbidden(
            "tenant_context_required",
            "A tenant context is required before starting an authorization flow.",
        ));
    }
    if client_id == "account-web" {
        return Ok(());
    }
    if workspace_id.is_none() {
        return Err(AppError::forbidden(
            "workspace_context_required",
            "Switch to a workspace before starting an authorization flow.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ensure_required_context;

    #[test]
    fn account_authorization_does_not_require_workspace_context() {
        ensure_required_context("account-web", Some(uuid::Uuid::new_v4()), None)
            .expect("Account is a principal-scoped resource");
    }

    #[test]
    fn workspace_products_keep_the_existing_context_requirement() {
        let error = ensure_required_context("cloud-web", None, None)
            .expect_err("Cloud authorization requires workspace context");

        assert_eq!(error.code, "workspace_context_required");
    }
}
