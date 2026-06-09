use crate::domains::auth::jwt::JwtService;
use crate::http::error::AppError;
use chrono::Utc;
use sqlx::postgres::PgPool;
use uuid::Uuid;

use crate::domains::oauth::flows::TokenView;

#[expect(
    clippy::too_many_arguments,
    reason = "OAuth token generation keeps scope, audience, and session context explicit."
)]
pub async fn generate_tokens(
    db: &PgPool,
    jwt: &JwtService,
    user_id: Uuid,
    tenant_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    workspace_id: Uuid,
    scope: String,
    audience: Option<String>,
    session_id: Option<Uuid>,
) -> Result<TokenView, AppError> {
    let workspace_region = sqlx::query_scalar::<_, Option<String>>(
        "SELECT data_region::text FROM workspaces WHERE id = $1",
    )
    .bind(workspace_id)
    .fetch_optional(db)
    .await?;
    let workspace_region = workspace_region.flatten();

    let tokens = jwt.generate_token_pair_with_session(
        user_id,
        Some(workspace_id),
        workspace_region,
        &scope,
        session_id,
        tenant_id,
        organization_id,
        Some("aal1"),
        Some(vec!["pwd".to_string()]),
        None,
        Some(Utc::now().timestamp()),
        None,
    )?;

    Ok(TokenView {
        access_token: tokens.access_token,
        token_type: tokens.token_type,
        expires_in: tokens.expires_in,
        refresh_token: Some(tokens.refresh_token),
        scope: audience.map_or(scope.clone(), |value| format!("{scope} audience:{value}")),
        authorization_details: Vec::new(),
        issued_token_type: None,
    })
}
