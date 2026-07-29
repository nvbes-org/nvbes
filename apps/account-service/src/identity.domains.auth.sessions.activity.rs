use axum::http::HeaderMap;
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::cache::current_session_ttl;
use crate::domains::auth::{audit, risk};
use crate::http::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityDecision {
    Allow,
    Observe,
    Throttle,
}

#[derive(Debug, Clone, Copy)]
pub struct ActivityAssessment {
    pub requests_in_window: u32,
    pub score: f64,
    pub decision: ActivityDecision,
    pub emit_event: bool,
}

pub fn assess(requests: u32) -> ActivityAssessment {
    let (score, decision) = if requests > 300 {
        (80.0, ActivityDecision::Throttle)
    } else if requests > 120 {
        (35.0, ActivityDecision::Observe)
    } else {
        (0.0, ActivityDecision::Allow)
    };
    ActivityAssessment {
        requests_in_window: requests,
        score,
        decision,
        emit_event: false,
    }
}

pub async fn enforce(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    session: &mut nvbes_redis::session::CachedSession,
    principal_id: Uuid,
    session_id: Uuid,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    let now = Utc::now();
    let session_key = session_id.to_string();
    let rate = nvbes_redis::rate_limit::check_rate_limit(
        redis,
        "auth",
        "session_activity",
        &session_key,
        300,
        60,
    )
    .await
    .map_err(|error| AppError::internal("session_activity_check_failed", error.to_string()))?;
    let requests = u32::try_from(rate.current).unwrap_or(u32::MAX);
    session.activity_request_count = requests;
    session.activity_window_started_at =
        Some(now - Duration::seconds(60_i64.saturating_sub(rate.ttl_seconds.min(60) as i64)));
    let mut assessment = assess(requests);
    if assessment.decision == ActivityDecision::Allow {
        return Ok(());
    }

    let event_gate = nvbes_redis::rate_limit::check_rate_limit(
        redis,
        "auth",
        "session_activity_event",
        &session_key,
        1,
        300,
    )
    .await
    .map_err(|error| AppError::internal("session_activity_event_gate_failed", error.to_string()))?;
    assessment.emit_event = event_gate.allowed;
    if assessment.emit_event {
        session.last_activity_risk_event_at = Some(now);
    }
    apply_risk_to_session(session, assessment);
    if assessment.emit_event {
        record_activity_event(db, session, principal_id, session_id, headers, assessment).await;
    }
    if assessment.decision == ActivityDecision::Throttle {
        let _ =
            nvbes_redis::session::set_session(redis, session, current_session_ttl(session)).await;
        return Err(AppError::too_many_requests(
            "session_activity_throttled",
            "This session is temporarily throttled due to unusual activity.",
            Some(60),
            None,
        ));
    }
    Ok(())
}

fn apply_risk_to_session(
    session: &mut nvbes_redis::session::CachedSession,
    assessment: ActivityAssessment,
) {
    session.risk_score = Some(session.risk_score.unwrap_or(0.0).max(assessment.score));
    session.risk_decision = Some(
        match assessment.decision {
            ActivityDecision::Allow => "allow",
            ActivityDecision::Observe => "step_up",
            ActivityDecision::Throttle => "deny",
        }
        .to_string(),
    );
    if assessment.decision != ActivityDecision::Allow {
        session.risk_confirmed_at = None;
        session.risk_confirmed_score = None;
    }
}

async fn record_activity_event(
    db: &PgPool,
    session: &nvbes_redis::session::CachedSession,
    principal_id: Uuid,
    session_id: Uuid,
    headers: &HeaderMap,
    assessment: ActivityAssessment,
) {
    let ip = crate::http::request::client_ip(headers);
    let user_agent = crate::http::request::user_agent(headers);
    let factors = serde_json::json!({
        "requests_in_minute": assessment.requests_in_window,
        "activity_score": assessment.score,
    });
    let _ = risk::record_event(
        db,
        risk::RiskEventInput {
            principal_id,
            session_id: Some(session_id),
            device_id: None,
            event_type: "session_activity_anomaly".to_string(),
            ip_address: ip.clone(),
            user_agent: user_agent.clone(),
            risk_score: assessment.score,
            risk_factors: factors.clone(),
            decision: if assessment.decision == ActivityDecision::Throttle {
                risk::RiskDecision::Deny
            } else {
                risk::RiskDecision::StepUp
            },
            metadata: serde_json::json!({
                "account_device_id": session.account_device_id,
                "window_seconds": 60,
            }),
        },
    )
    .await;
    let _ = audit::record_auth_event(
        db,
        audit::AuthAuditInput {
            principal_id,
            action: "auth.session_activity_anomaly",
            target_type: "session",
            target_id: Some(session_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: factors,
        },
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sustained_api_burst_is_throttled() {
        let assessment = assess(301);

        assert_eq!(assessment.decision, ActivityDecision::Throttle);
    }

    #[test]
    fn normal_request_volume_is_allowed() {
        let assessment = assess(1);

        assert_eq!(assessment.requests_in_window, 1);
        assert_eq!(assessment.decision, ActivityDecision::Allow);
    }
}
