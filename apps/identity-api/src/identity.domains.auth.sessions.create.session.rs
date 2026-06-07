use chrono::Utc;
use nvbes_core::config::AppConfig;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::auth::jwt::JwtService;
use crate::domains::auth::{
    db as auth_db, email_verification, mfa, password, risk,
    sessions::cache::{
        cached_session_from_login, current_session_ttl, session_view_from_cached_session,
    },
    sessions_context,
    types::{LoginResult, UserView},
};
use crate::http::error::AppError;

pub struct LoginSessionContext {
    pub email: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub device_fingerprint: Option<Value>,
    pub amr: Vec<String>,
    pub acr: &'static str,
}

pub async fn create_session_for_principal(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    config: &AppConfig,
    principal_id: Uuid,
    context: LoginSessionContext,
) -> Result<LoginResult, AppError> {
    let user = auth_db::fetch_user_record(db, principal_id).await?;
    let tenant_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT tenant_id
        FROM principals
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;
    let (workspace_id, organization_id, workspace_region) =
        sessions_context::first_workspace_context(db, principal_id).await?;
    let verification_resend_available_at = if user.email_verified_at.is_some() {
        None
    } else {
        let resend_token =
            nvbes_redis::email_verification::latest_unconsumed_email_verification_token_for_principal(
                redis,
                principal_id,
            )
            .await
            .map_err(|err| AppError::internal("email_verification_token_read_failed", &err.to_string()))?;

        resend_token.map(|token| {
            email_verification::verification_resend_available_at(
                token.created_at,
                config.auth_verification_resend_cooldown_seconds,
            )
        })
    };

    let session_id = Uuid::new_v4();
    let workspace_region_for_cache = workspace_region.clone();
    let amr_for_cache = context.amr.clone();
    let token_pair = jwt.generate_token_pair_with_session(
        principal_id,
        workspace_id,
        workspace_region.clone(),
        "openid profile email offline_access",
        Some(session_id),
        Some(tenant_id),
        organization_id,
        Some(context.acr),
        Some(context.amr.clone()),
        None,
        Some(Utc::now().timestamp()),
        None,
    )?;

    let now = Utc::now();
    let cached_session = cached_session_from_login(
        session_id,
        principal_id,
        Some(tenant_id),
        organization_id,
        workspace_id,
        workspace_region_for_cache,
        None,
        password::token_hash(&token_pair.access_token),
        Some(context.acr.to_string()),
        amr_for_cache,
        now,
        now,
        context.ip.clone(),
        context.user_agent.clone(),
        now + chrono::Duration::hours(config.auth_session_ttl_hours),
    );
    let _ = nvbes_redis::session::set_session(
        redis,
        &cached_session,
        current_session_ttl(&cached_session),
    )
    .await;

    let (score, decision, factors) = risk::current_state_summary(db, principal_id).await?;
    let _ = risk::record_event(
        db,
        risk::RiskEventInput {
            principal_id,
            session_id: Some(session_id),
            device_id: None,
            event_type: "login_success".to_string(),
            ip_address: context.ip,
            user_agent: context.user_agent,
            risk_score: score,
            risk_factors: factors.clone(),
            decision,
            metadata: serde_json::json!({
                "tenant_id": tenant_id,
                "workspace_id": workspace_id,
            }),
        },
    )
    .await;

    Ok(LoginResult {
        user: UserView {
            id: principal_id,
            email: context.email,
            display_name: user.display_name,
            firstname: user.firstname,
            lastname: user.lastname,
            username: user.username,
            birthdate: user.birthdate,
            region: user.region,
            email_verified: user.email_verified_at.is_some(),
            mfa_enabled: mfa::has_active_factor(db, principal_id).await?,
            created_at: user.created_at,
        },
        session: session_view_from_cached_session(&cached_session, true),
        session_token: token_pair.access_token,
        verification_resend_available_at,
    })
}
