use chrono::Utc;
use nvbes_core::config::AppConfig;
use nvbes_core::limiter::RateLimiter;
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::cache::{
    cached_session_from_login, current_session_ttl, session_view_from_cached_session,
};
use crate::domains::auth::jwt::JwtService;
use crate::domains::auth::{
    db as auth_db, email_verification, mfa, password, risk, sessions_context, types::*,
};
use crate::http::error::AppError;

pub struct VerifiedPrimaryLogin {
    pub principal_id: Uuid,
    pub email: String,
}

pub struct LoginSessionContext {
    pub email: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub device_fingerprint: Option<Value>,
    pub amr: Vec<String>,
    pub acr: &'static str,
}

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

pub async fn verify_primary_credentials(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    input: &LoginInput,
) -> Result<VerifiedPrimaryLogin, AppError> {
    let email = password::normalize_email(&input.email);
    RateLimiter::new(redis.clone())
        .check("login", &email, 12, std::time::Duration::from_secs(300))
        .await?;

    let row = sqlx::query(
        r#"
        SELECT
          u.principal_id,
          u.email,
          u.firstname,
          u.lastname,
          u.username,
          u.birthdate,
          u.region,
          u.password_hash,
          u.email_verified_at,
          u.password_last_changed_at,
          u.created_at,
          p.tenant_id
        FROM users u
        INNER JOIN principals p ON p.id = u.principal_id
        WHERE lower(u.email) = lower($1)
        LIMIT 1
        "#,
    )
    .bind(&email)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::unauthorized("invalid_credentials", "Invalid email or password."))?;

    let principal_id: Uuid = row.get("principal_id");

    if let Some(max_age_days) = config.auth_password_max_age_days {
        let last_changed: Option<chrono::DateTime<chrono::Utc>> =
            row.get("password_last_changed_at");
        let expired = last_changed.map_or(true, |lc| {
            lc + chrono::Duration::days(max_age_days) <= chrono::Utc::now()
        });
        if expired {
            return Err(AppError::forbidden(
                "password_expired",
                "Your password has expired and must be changed.",
            ));
        }
    }
    let (risk_score, risk_decision, risk_factors) =
        risk::current_state_summary(db, principal_id).await?;
    if matches!(
        risk_decision,
        risk::RiskDecision::Deny | risk::RiskDecision::Lock
    ) {
        let _ = risk::record_event(
            db,
            risk::RiskEventInput {
                principal_id,
                session_id: None,
                device_id: None,
                event_type: "login_blocked".to_string(),
                ip_address: input.ip.clone(),
                user_agent: input.user_agent.clone(),
                risk_score,
                risk_factors: risk_factors.clone(),
                decision: risk_decision,
                metadata: serde_json::json!({
                    "email": email,
                    "reason": "suspicious_activity",
                }),
            },
        )
        .await;

        return Err(AppError::forbidden(
            "risk_policy_blocked",
            "This account is temporarily blocked due to suspicious activity.",
        ));
    }

    let password_hash: Option<String> = row.get("password_hash");
    let stored_hash = password_hash.ok_or_else(|| {
        AppError::unauthorized("invalid_credentials", "Invalid email or password.")
    })?;

    crate::domains::auth::login_delay::apply_login_delay(db, principal_id).await?;

    if let Err(err) = password::verify_password(&stored_hash, &input.password) {
        let _ = risk::record_event(
            db,
            risk::RiskEventInput {
                principal_id,
                session_id: None,
                device_id: None,
                event_type: "login_failed".to_string(),
                ip_address: input.ip.clone(),
                user_agent: input.user_agent.clone(),
                risk_score: 25.0,
                risk_factors: serde_json::json!({
                    "reason": "invalid_password"
                }),
                decision: risk::RiskDecision::StepUp,
                metadata: serde_json::json!({
                    "email": email,
                }),
            },
        )
        .await;
        return Err(err);
    }

    Ok(VerifiedPrimaryLogin {
        principal_id,
        email,
    })
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
