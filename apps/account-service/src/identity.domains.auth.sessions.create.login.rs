use nvbes_core::config::AppConfig;
use sqlx::PgPool;

use super::{LoginSessionContext, create_session_for_principal, verify_primary_credentials};
use crate::domains::auth::sessions::cookie_theft::SessionRequestProfile;
use crate::domains::auth::types::{LoginInput, LoginResult};
use crate::http::error::AppError;

pub async fn login(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    input: LoginInput,
) -> Result<LoginResult, AppError> {
    let verified = verify_primary_credentials(db, redis, config, &input).await?;
    create_session_for_principal(
        db,
        redis,
        config,
        verified.principal_id,
        LoginSessionContext {
            email: verified.email,
            request_profile: SessionRequestProfile {
                ip: input.ip.clone(),
                user_agent: input.user_agent.clone(),
                accept_language: None,
                accept: None,
                accept_encoding: None,
                sec_fetch_site: None,
                sec_fetch_mode: None,
                sec_fetch_dest: None,
                ua_client_hints: Default::default(),
            },
            installation_token: None,
            ip: input.ip,
            user_agent: input.user_agent,
            device_fingerprint: input.device_fingerprint,
            amr: vec!["pwd".to_string()],
            acr: "aal1",
        },
    )
    .await
}
