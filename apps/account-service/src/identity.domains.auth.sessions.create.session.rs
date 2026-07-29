use chrono::Utc;
use nvbes_core::config::AppConfig;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::auth::{
    audit::{AuthAuditInput, record_auth_event},
    db as auth_db, email_verification, mfa, registration_enrollment, risk,
    sessions::cache::{
        cached_session_from_login, configure_idle_timeout, current_session_ttl,
        session_view_from_cached_session,
    },
    sessions::cookie_theft::{SessionRequestProfile, apply_profile},
    sessions::token,
    types::{LoginResult, UserView},
};
use crate::http::error::AppError;

use super::geo_impl::record_login_geo_signal;

pub struct LoginSessionContext {
    pub email: String,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub request_profile: SessionRequestProfile,
    pub installation_token: Option<String>,
    pub device_fingerprint: Option<Value>,
    pub amr: Vec<String>,
    pub acr: &'static str,
}

pub async fn create_session_for_principal(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
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
    let verification_resend_available_at = if user.email_verified_at.is_some() {
        None
    } else {
        let resend_token =
            nvbes_redis::email_verification::latest_unconsumed_email_verification_token_for_principal(
                redis,
                principal_id,
            )
            .await
            .map_err(|err| AppError::internal("email_verification_token_read_failed", err.to_string()))?;

        resend_token.map(|token| {
            email_verification::verification_resend_available_at(
                token.created_at,
                config.auth_verification_resend_cooldown_seconds,
            )
        })
    };
    let registration_enrollment_token = if user.email_verified_at.is_none() {
        Some(registration_enrollment::issue(db, config, principal_id).await?)
    } else {
        None
    };

    let session_id = Uuid::new_v4();
    let browser_session_token = token::issue(session_id);
    let amr_for_cache = context.amr.clone();

    let now = Utc::now();
    let device_assessment = crate::domains::auth::device_trust::assess_authenticated_device(
        db,
        principal_id,
        context.installation_token.as_deref(),
        context.device_fingerprint.as_ref(),
        context.ip.as_deref(),
        context.user_agent.as_deref(),
        &context.amr,
        context.acr,
        &config.jwt_secret,
    )
    .await?;
    let mut cached_session = cached_session_from_login(
        session_id,
        principal_id,
        Some(tenant_id),
        None,
        None,
        None,
        None,
        token::hash(&browser_session_token),
        Some(context.acr.to_string()),
        amr_for_cache,
        now,
        now,
        context.ip.clone(),
        context.user_agent.clone(),
        now + chrono::Duration::hours(config.auth_session_ttl_hours),
    );
    configure_idle_timeout(
        &mut cached_session,
        config.auth_session_idle_ttl_minutes,
        now,
    );
    apply_profile(&mut cached_session, &context.request_profile);
    let session_client = crate::domains::auth::user_agent::parse(
        context.user_agent.as_deref(),
        context.request_profile.ua_client_hints.brands.as_deref(),
    );
    cached_session.account_device_id = Some(device_assessment.device_id.to_string());
    cached_session.device_trust_level = Some(device_assessment.trust_level.clone());
    cached_session.device_trust_score = Some(device_assessment.trust_score);
    let device_fingerprint_hash = context
        .device_fingerprint
        .as_ref()
        .and_then(crate::domains::auth::device_trust::profile::CoarseDeviceProfile::from_value)
        .map(|profile| profile.keyed_hash(&config.jwt_secret));
    let country = context
        .device_fingerprint
        .as_ref()
        .and_then(|value| value.get("country").and_then(Value::as_str));
    let context_signals = risk::evaluate_login_context_signals(
        db,
        principal_id,
        context.ip.as_deref(),
        country,
        device_fingerprint_hash.as_deref(),
    )
    .await?;
    let (mut score, mut decision, mut factors) =
        risk::current_state_summary(db, principal_id).await?;
    score += context_signals.score_delta;
    score += device_assessment.risk_delta;
    if (context_signals.score_delta > 0.0 || device_assessment.risk_delta > 0.0)
        && matches!(decision, risk::RiskDecision::Allow)
    {
        decision = risk::RiskDecision::StepUp;
    }
    if let Some(object) = factors.as_object_mut() {
        object.insert(
            "new_ip".to_string(),
            serde_json::json!(context_signals.new_ip),
        );
        object.insert(
            "new_device".to_string(),
            serde_json::json!(context_signals.new_device),
        );
        object.insert(
            "unusual_country".to_string(),
            serde_json::json!(context_signals.unusual_country),
        );
        object.insert(
            "known_device".to_string(),
            serde_json::json!(device_assessment.known_device),
        );
        object.insert(
            "device_profile_match".to_string(),
            serde_json::json!(device_assessment.profile_match),
        );
        object.insert(
            "device_trust_score".to_string(),
            serde_json::json!(device_assessment.trust_score),
        );
    }
    let geo_resolution = record_login_geo_signal(
        db,
        config,
        principal_id,
        context.ip.as_deref(),
        country,
        &mut score,
        &mut decision,
        &mut factors,
    )
    .await;
    cached_session.geo_country_code = geo_resolution
        .as_ref()
        .and_then(|resolution| resolution.location.as_ref())
        .map(|location| location.country_code.clone());
    cached_session.risk_score = Some(score);
    cached_session.risk_decision = Some(decision.as_str().to_string());
    nvbes_redis::session::set_session(redis, &cached_session, current_session_ttl(&cached_session))
        .await
        .map_err(|err| AppError::internal("session_cache_write_failed", err.to_string()))?;
    crate::domains::auth::sessions::db::insert_session_db(db, &cached_session)
        .await
        .map_err(|err| AppError::internal("session_db_write_failed", err.to_string()))?;
    let _ = risk::record_event(
        db,
        risk::RiskEventInput {
            principal_id,
            session_id: Some(session_id),
            device_id: None,
            event_type: "login_success".to_string(),
            ip_address: context.ip.clone(),
            user_agent: context.user_agent.clone(),
            risk_score: score,
            risk_factors: factors.clone(),
            decision,
            metadata: serde_json::json!({
                "tenant_id": tenant_id,
                "country": country,
                "geo_country_code": geo_resolution.as_ref().and_then(|resolution| {
                    resolution
                        .location
                        .as_ref()
                        .map(|location| location.country_code.as_str())
                }),
                "geo_source": geo_resolution.as_ref().map(|resolution| resolution.source.as_str()),
                "geo_confidence": geo_resolution.as_ref().map(|resolution| resolution.confidence.as_str()),
                "geo_network_kind": geo_resolution.as_ref().map(|resolution| resolution.network_kind.as_str()),
                "geo_risk_score": geo_resolution.as_ref().map(|resolution| resolution.risk_score),
                "geo_risk_labels": geo_resolution.as_ref().map(|resolution| &resolution.risk_labels),
                "device_fingerprint_hash": device_fingerprint_hash,
                "account_device_id": device_assessment.device_id,
                "device_trust_level": device_assessment.trust_level,
                "device_trust_score": device_assessment.trust_score,
                "client": session_client,
            }),
        },
    )
    .await;
    let _ = record_auth_event(
        db,
        AuthAuditInput {
            principal_id,
            action: "auth.login_success",
            target_type: "session",
            target_id: Some(session_id),
            ip: context.ip.as_deref(),
            user_agent: context.user_agent.as_deref(),
            metadata: serde_json::json!({
                "tenant_id": tenant_id,
                "acr": context.acr,
                "amr": context.amr.clone(),
                "risk_score": score,
                "risk_decision": decision.as_str(),
                "geo_country_code": geo_resolution.as_ref().and_then(|resolution| {
                    resolution
                        .location
                        .as_ref()
                        .map(|location| location.country_code.as_str())
                }),
                "geo_source": geo_resolution.as_ref().map(|resolution| resolution.source.as_str()),
                "geo_confidence": geo_resolution.as_ref().map(|resolution| resolution.confidence.as_str()),
                "geo_network_kind": geo_resolution.as_ref().map(|resolution| resolution.network_kind.as_str()),
                "geo_risk_score": geo_resolution.as_ref().map(|resolution| resolution.risk_score),
                "geo_risk_labels": geo_resolution.as_ref().map(|resolution| &resolution.risk_labels),
                "client": session_client,
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
        browser_session_token,
        device_cookie_token: device_assessment.cookie_token,
        verification_resend_available_at,
        registration_enrollment_token,
    })
}
