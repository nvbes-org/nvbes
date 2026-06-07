use nvbes_core::config::AppConfig;
use sqlx::PgPool;

use super::{LoginSessionContext, create_session_for_principal, verify_primary_credentials};
use crate::domains::auth::jwt::JwtService;
use crate::domains::auth::types::{LoginInput, LoginResult};
use crate::http::error::AppError;

pub async fn login(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    config: &AppConfig,
    input: LoginInput,
) -> Result<LoginResult, AppError> {
    let verified = verify_primary_credentials(db, redis, config, &input).await?;
    create_session_for_principal(
        db,
        redis,
        jwt,
        config,
        verified.principal_id,
        LoginSessionContext {
            email: verified.email,
            ip: input.ip,
            user_agent: input.user_agent,
            device_fingerprint: input.device_fingerprint,
            amr: vec!["pwd".to_string()],
            acr: "aal1",
        },
    )
    .await
}
