use std::sync::Arc;
use uuid::Uuid;

use nvbes_identity_service::tokens::TokenService;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedPrincipal {
    pub id: Uuid,
    pub scopes: Vec<String>,
    pub client_id: Option<String>,
    pub step_up: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum PortError {
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("internal port error: {0}")]
    Internal(String),
}

pub trait IdentityPort: Send + Sync {
    fn introspect_token(
        &self,
        token: &str,
        audience: &str,
    ) -> Result<AuthenticatedPrincipal, PortError>;
}

pub struct InProcessIdentityAdapter {
    token_service: Arc<TokenService>,
}

impl InProcessIdentityAdapter {
    pub fn new(token_service: Arc<TokenService>) -> Self {
        Self { token_service }
    }
}

impl IdentityPort for InProcessIdentityAdapter {
    fn introspect_token(
        &self,
        token: &str,
        audience: &str,
    ) -> Result<AuthenticatedPrincipal, PortError> {
        let claims = self
            .token_service
            .verify(token, audience)
            .map_err(|e| PortError::Unauthorized(e.to_string()))?;

        let id = Uuid::parse_str(&claims.sub)
            .map_err(|e| PortError::InvalidArgument(format!("invalid principal UUID: {e}")))?;

        let scopes = claims.scope.split_whitespace().map(String::from).collect();

        Ok(AuthenticatedPrincipal {
            id,
            scopes,
            client_id: Some(claims.client_id),
            step_up: claims.step_up_time.is_some(),
        })
    }
}

pub trait AccountPort: Send + Sync {
    fn is_team_member(
        &self,
        user_id: Uuid,
        team_id: Uuid,
    ) -> impl std::future::Future<Output = Result<bool, PortError>> + Send;
}

pub struct InProcessAccountAdapter {
    pool: sqlx::PgPool,
}

impl InProcessAccountAdapter {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

impl AccountPort for InProcessAccountAdapter {
    async fn is_team_member(&self, user_id: Uuid, team_id: Uuid) -> Result<bool, PortError> {
        let exists = sqlx::query_scalar::<_, bool>(
            // nvbes-allow-runtime-sql
            r#"SELECT EXISTS(
                SELECT 1 FROM account_team_memberships
                WHERE team_id = $1 AND principal_id = $2
            )"#,
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PortError::Internal(e.to_string()))?;

        Ok(exists)
    }
}

pub use nvbes_consent::port::ConsentPort as UnifiedConsentPort;
pub use nvbes_consent::service::ConsentService as InProcessConsentAdapter;
