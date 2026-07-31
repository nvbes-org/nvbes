use axum::http::HeaderMap;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use super::cache::{auth_context_from_cached_session, current_session_ttl, refresh_last_seen};
use super::cookie_theft::{self, CookieTheftDecision, SessionRequestProfile};
use crate::domains::auth::jwt::JwtService;
use crate::domains::auth::types::AuthContext;
use crate::domains::auth::{audit, db as auth_db, mfa, risk, sessions::cache};
use crate::http::error::AppError;

pub async fn authenticate(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    token: &str,
) -> Result<AuthContext, AppError> {
    authenticate_impl(db, redis, jwt, token, None).await
}

pub async fn authenticate_with_request(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    token: &str,
    headers: &HeaderMap,
) -> Result<AuthContext, AppError> {
    authenticate_impl(db, redis, jwt, token, Some(headers)).await
}

async fn authenticate_impl(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    token: &str,
    headers: Option<&HeaderMap>,
) -> Result<AuthContext, AppError> {
    let claims = jwt.decode_token(token, "access")?;
    if claims.token_type != "access" {
        return Err(AppError::unauthorized(
            "invalid_token_type",
            "Access token is required.",
        ));
    }

    let principal_id = Uuid::parse_str(&claims.sub)
        .map_err(|e| AppError::unauthorized("invalid_subject", format!("{}", e)))?;
    let session_id = Uuid::parse_str(&claims.sid)
        .map_err(|e| AppError::unauthorized("invalid_session", format!("{}", e)))?;
    let mut session = get_cached_session(db, redis, session_id)
        .await?
        .ok_or_else(|| AppError::unauthorized("session_not_found", "Session not found."))?;
    if session.principal_id != principal_id.to_string() {
        return Err(AppError::unauthorized(
            "session_not_found",
            "Session not found.",
        ));
    }
    if nvbes_redis::session::is_session_revoked(redis, &session.session_id)
        .await
        .map_err(|err| {
            AppError::internal("redis_session_revocation_lookup_failed", err.to_string())
        })?
        || cache::is_expired(&session, Utc::now())
    {
        let _ =
            nvbes_redis::session::delete_session(redis, &session.principal_id, &session.session_id)
                .await;
        return Err(AppError::unauthorized(
            "session_expired",
            "Session expired.",
        ));
    }

    if let Some(headers) = headers {
        enforce_cookie_theft_mitigation(db, redis, &mut session, principal_id, session_id, headers)
            .await?;
        super::activity::enforce(db, redis, &mut session, principal_id, session_id, headers)
            .await?;
    }

    refresh_last_seen(&mut session);
    let ttl = current_session_ttl(&session);
    let _ = nvbes_redis::session::set_session(redis, &session, ttl).await;

    let user = build_user_record_from_cache(&session, db, principal_id).await?;
    if user.status != "active" {
        return Err(AppError::forbidden(
            "account_disabled",
            "This account is not active.",
        ));
    }

    Ok(auth_context_from_cached_session(
        &session,
        &user,
        mfa::has_active_factor(db, principal_id).await?,
        claims.scope,
        claims.cnf.and_then(|confirmation| confirmation.jkt),
    ))
}

pub(super) async fn enforce_cookie_theft_mitigation(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    session: &mut cache::CachedSession,
    principal_id: Uuid,
    session_id: Uuid,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    let profile = SessionRequestProfile::from_headers(headers);
    if !cookie_theft::has_detection_profile(session) {
        cookie_theft::apply_profile(session, &profile);
        return Ok(());
    }

    let assessment = cookie_theft::assess(session, &profile);
    let baseline_client = crate::domains::auth::user_agent::parse(
        session.user_agent.as_deref(),
        session.sec_ch_ua.as_deref(),
    );
    let current_client = crate::domains::auth::user_agent::parse(
        profile.user_agent.as_deref(),
        profile.ua_client_hints.brands.as_deref(),
    );
    session.cookie_theft_risk_score = Some(assessment.score);
    session.risk_score = Some(session.risk_score.unwrap_or(0.0).max(assessment.score));
    session.risk_decision = Some(
        match assessment.decision {
            CookieTheftDecision::Allow => "allow",
            CookieTheftDecision::StepUp => "step_up",
            CookieTheftDecision::Reauthenticate => "deny",
        }
        .to_string(),
    );
    if assessment.decision != CookieTheftDecision::Allow {
        session.risk_confirmed_at = None;
        session.risk_confirmed_score = None;
    }
    if assessment.decision == CookieTheftDecision::Allow {
        return Ok(());
    }

    let decision = match assessment.decision {
        CookieTheftDecision::Allow => risk::RiskDecision::Allow,
        CookieTheftDecision::StepUp => risk::RiskDecision::StepUp,
        CookieTheftDecision::Reauthenticate => risk::RiskDecision::Deny,
    };
    session.cookie_theft_detected_at = Some(assessment.assessed_at);
    let factors = serde_json::json!({
        "cookie_theft_risk_score": assessment.score,
        "factors": assessment.factors,
        "decision": format!("{:?}", assessment.decision),
    });
    let _ = risk::record_event(
        db,
        risk::RiskEventInput {
            principal_id,
            session_id: Some(session_id),
            device_id: None,
            event_type: "cookie_theft_suspected".to_string(),
            ip_address: profile.ip.clone(),
            user_agent: profile.user_agent.clone(),
            risk_score: assessment.score,
            risk_factors: factors.clone(),
            decision,
            metadata: serde_json::json!({
                "session_id": session_id,
                "baseline_ip": session.ip.clone(),
                "baseline_user_agent": session.user_agent.clone(),
                "baseline_client": baseline_client,
                "current_client": current_client,
                "current_accept_language": profile.accept_language,
                "current_sec_fetch_site": profile.sec_fetch_site,
            }),
        },
    )
    .await;
    let _ = audit::record_auth_event(
        db,
        audit::AuthAuditInput {
            principal_id,
            action: "auth.cookie_theft_suspected",
            target_type: "session",
            target_id: Some(session_id),
            ip: profile.ip.as_deref(),
            user_agent: profile.user_agent.as_deref(),
            metadata: factors,
        },
    )
    .await;

    if assessment.decision == CookieTheftDecision::Reauthenticate {
        revoke_suspicious_session(db, redis, principal_id, session_id).await?;
        return Err(AppError::unauthorized(
            "session_reauthentication_required",
            "Session reauthentication is required.",
        ));
    }

    Ok(())
}

pub(super) async fn revoke_suspicious_session(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    session_id: Uuid,
) -> Result<(), AppError> {
    crate::domains::oauth::security_events::enqueue_session_revoked(
        db,
        redis,
        principal_id,
        session_id,
    )
    .await?;
    let _ =
        nvbes_redis::refresh_token::revoke_session_refresh_tokens(redis, principal_id, session_id)
            .await;
    nvbes_redis::session::delete_session(redis, &principal_id.to_string(), &session_id.to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_revoke_failed", err.to_string()))
}

pub async fn authenticate_bearer(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    headers: &HeaderMap,
) -> Result<AuthContext, AppError> {
    let token = crate::http::request::bearer_token(headers)?;
    authenticate(db, redis, jwt, &token).await
}

pub async fn try_authenticate_bearer(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    headers: &HeaderMap,
) -> Option<AuthContext> {
    let token = crate::http::request::bearer_token(headers).ok()?;
    authenticate(db, redis, jwt, &token).await.ok()
}

pub async fn authenticate_verified_bearer(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    jwt: &JwtService,
    headers: &HeaderMap,
) -> Result<AuthContext, AppError> {
    let auth = authenticate_bearer(db, redis, jwt, headers).await?;
    if auth.email_verified_at.is_none() {
        return Err(AppError::forbidden(
            "email_not_verified",
            "Verify your email address before performing this action.",
        ));
    }
    Ok(auth)
}

pub(super) async fn get_cached_session(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    session_id: Uuid,
) -> Result<Option<cache::CachedSession>, AppError> {
    if let Some(session) = nvbes_redis::session::get_session(redis, &session_id.to_string())
        .await
        .map_err(|err| AppError::internal("redis_session_cache_read_failed", err.to_string()))?
    {
        return Ok(Some(session));
    }

    let db_session = super::db::fetch_session_db(db, session_id)
        .await
        .map_err(|err| AppError::internal("db_session_cache_read_failed", err.to_string()))?;

    if let Some(ref session) = db_session {
        let ttl = current_session_ttl(session);
        let _ = nvbes_redis::session::set_session(redis, session, ttl).await;
    }

    Ok(db_session)
}

pub(super) async fn build_user_record_from_cache(
    session: &cache::CachedSession,
    db: &PgPool,
    principal_id: Uuid,
) -> Result<auth_db::UserRecord, AppError> {
    let user = auth_db::fetch_user_record(db, principal_id).await?;
    if session.principal_id != user.principal_id.to_string() {
        return Err(AppError::unauthorized(
            "session_not_found",
            "Session not found.",
        ));
    }
    Ok(user)
}
