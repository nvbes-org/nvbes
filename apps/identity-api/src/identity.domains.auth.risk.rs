use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::http::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskDecision {
    Allow,
    StepUp,
    Deny,
    Lock,
}

impl RiskDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::StepUp => "step_up",
            Self::Deny => "deny",
            Self::Lock => "lock",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RiskEventInput {
    pub principal_id: Uuid,
    pub session_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
    pub event_type: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub risk_score: f64,
    pub risk_factors: Value,
    pub decision: RiskDecision,
    pub metadata: Value,
}

pub async fn record_event(db: &PgPool, input: RiskEventInput) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO risk_events (
          principal_id, session_id, device_id, event_type, ip_address, user_agent,
          risk_score, risk_factors, decision, metadata, created_at
        )
        VALUES ($1, $2, $3, $4, $5::inet, $6, $7, $8, $9, $10, NOW())
        "#,
    )
    .bind(input.principal_id)
    .bind(input.session_id)
    .bind(input.device_id)
    .bind(input.event_type)
    .bind(input.ip_address.as_deref())
    .bind(input.user_agent.as_deref())
    .bind(input.risk_score)
    .bind(sqlx::types::Json(input.risk_factors))
    .bind(input.decision.as_str())
    .bind(sqlx::types::Json(input.metadata))
    .execute(db)
    .await?;

    Ok(())
}

pub async fn evaluate_principal_risk(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<(f64, RiskDecision, Value), AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          COUNT(*) FILTER (WHERE created_at >= NOW() - INTERVAL '15 minutes') AS recent_events,
          COUNT(*) FILTER (WHERE event_type IN ('login_failed', 'password_reset_requested', 'webauthn_failed')
                           AND created_at >= NOW() - INTERVAL '15 minutes') AS recent_security_events,
          COUNT(*) FILTER (WHERE decision IN ('deny', 'lock')
                           AND created_at >= NOW() - INTERVAL '24 hours') AS recent_denials,
          COUNT(*) FILTER (WHERE event_type = 'login_failed'
                           AND created_at >= NOW() - INTERVAL '30 minutes') AS recent_login_failures,
          COUNT(*) FILTER (WHERE event_type = 'password_reset_requested'
                           AND created_at >= NOW() - INTERVAL '24 hours') AS recent_reset_requests
        FROM risk_events
        WHERE principal_id = $1
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;

    let recent_security_events: i64 = row.get("recent_security_events");
    let recent_denials: i64 = row.get("recent_denials");
    let recent_login_failures: i64 = row.get("recent_login_failures");
    let recent_reset_requests: i64 = row.get("recent_reset_requests");
    let recent_events: i64 = row.get("recent_events");

    let mut score = 0.0_f64;
    let mut factors = serde_json::Map::new();

    if recent_login_failures > 0 {
        score += (recent_login_failures as f64).min(10.0) * 8.0;
        factors.insert(
            "login_failures".to_string(),
            serde_json::json!(recent_login_failures),
        );
    }
    if recent_reset_requests > 2 {
        score += 18.0;
        factors.insert(
            "reset_burst".to_string(),
            serde_json::json!(recent_reset_requests),
        );
    }
    if recent_security_events > 3 {
        score += 20.0;
        factors.insert(
            "security_event_burst".to_string(),
            serde_json::json!(recent_security_events),
        );
    }
    if recent_denials > 0 {
        score += (recent_denials as f64) * 25.0;
        factors.insert(
            "recent_denials".to_string(),
            serde_json::json!(recent_denials),
        );
    }
    if recent_events > 40 {
        score += 10.0;
        factors.insert("event_volume".to_string(), serde_json::json!(recent_events));
    }

    let decision = if score >= 80.0 {
        RiskDecision::Lock
    } else if score >= 50.0 {
        RiskDecision::Deny
    } else if score >= 25.0 {
        RiskDecision::StepUp
    } else {
        RiskDecision::Allow
    };

    factors.insert("risk_score".to_string(), serde_json::json!(score));
    factors.insert("decision".to_string(), serde_json::json!(decision.as_str()));
    factors.insert(
        "window".to_string(),
        serde_json::json!({
            "recent_minutes": 15,
            "reset_window_hours": 24
        }),
    );

    Ok((score, decision, serde_json::Value::Object(factors)))
}

pub async fn should_lock_password_reset(db: &PgPool, principal_id: Uuid) -> Result<bool, AppError> {
    let row = sqlx::query(
        r#"
        SELECT COUNT(*) AS count
        FROM risk_events
        WHERE principal_id = $1
          AND event_type = 'password_reset_requested'
          AND created_at >= NOW() - INTERVAL '30 minutes'
        "#,
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;

    let count: i64 = row.get("count");
    Ok(count >= 4)
}

pub async fn current_state_summary(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<(f64, RiskDecision, Value), AppError> {
    let (mut score, decision, factors) = evaluate_principal_risk(db, principal_id).await?;
    if score.is_nan() || !score.is_finite() {
        score = 0.0;
    }
    Ok((score, decision, factors))
}
