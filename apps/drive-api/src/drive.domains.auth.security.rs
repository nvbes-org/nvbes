use chrono::{DateTime, Duration as ChronoDuration, Utc};
use nvbes_core::auth::Aal;

use super::types::AuthContext;
use crate::http::error::AppError;

pub async fn require_recent_step_up(
    _db: &sqlx::PgPool,
    auth: &AuthContext,
) -> Result<(), AppError> {
    if !has_recent_step_up(auth, Aal::Aal2, step_up_ttl_minutes(), Utc::now()) {
        return Err(AppError::forbidden(
            "step_up_required",
            "Please verify again before continuing.",
        ));
    }

    Ok(())
}

fn has_recent_step_up(
    auth: &AuthContext,
    required: Aal,
    ttl_minutes: i64,
    now: DateTime<Utc>,
) -> bool {
    let current_level = auth
        .acr
        .as_deref()
        .and_then(|acr| acr.parse::<Aal>().ok())
        .unwrap_or(Aal::Aal1);
    let Some(auth_time) = auth.auth_time else {
        return false;
    };

    current_level >= required && auth_time + ChronoDuration::minutes(ttl_minutes) > now
}

fn step_up_ttl_minutes() -> i64 {
    std::env::var("NVBES_AUTH_STEP_UP_TTL_MINUTES")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(15)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    use crate::domains::auth::types::AuthPrincipalKind;

    fn auth_context(acr: Option<&str>, auth_time: Option<DateTime<Utc>>) -> AuthContext {
        AuthContext {
            principal_id: Uuid::nil(),
            principal_kind: AuthPrincipalKind::User,
            user_id: Uuid::nil(),
            email_verified_at: None,
            session_id: Uuid::nil(),
            tenant_id: None,
            organization_id: None,
            workspace_id: None,
            scope: String::new(),
            role: None,
            amr: Vec::new(),
            actor: None,
            acr: acr.map(str::to_owned),
            auth_time,
        }
    }

    #[test]
    fn has_recent_step_up_accepts_fresh_aal2_session() {
        let now = Utc.timestamp_opt(1_700_000_000, 0).single().unwrap();
        let auth = auth_context(
            Some("aal2"),
            Some(Utc.timestamp_opt(1_699_999_200, 0).single().unwrap()),
        );

        assert!(has_recent_step_up(&auth, Aal::Aal2, 15, now));
    }

    #[test]
    fn has_recent_step_up_rejects_stale_auth_time() {
        let now = Utc.timestamp_opt(1_700_000_000, 0).single().unwrap();
        let auth = auth_context(
            Some("aal2"),
            Some(Utc.timestamp_opt(1_699_998_000, 0).single().unwrap()),
        );

        assert!(!has_recent_step_up(&auth, Aal::Aal2, 15, now));
    }

    #[test]
    fn has_recent_step_up_rejects_low_assurance() {
        let now = Utc.timestamp_opt(1_700_000_000, 0).single().unwrap();
        let auth = auth_context(
            Some("aal1"),
            Some(Utc.timestamp_opt(1_699_999_900, 0).single().unwrap()),
        );

        assert!(!has_recent_step_up(&auth, Aal::Aal2, 15, now));
    }

    #[tokio::test]
    async fn require_recent_step_up_returns_forbidden_when_stale_or_missing() {
        let auth = auth_context(Some("aal1"), Some(Utc::now()));
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/dummy").unwrap();
        let err = require_recent_step_up(&pool, &auth)
            .await
            .expect_err("should fail");
        assert_eq!(err.status, axum::http::StatusCode::FORBIDDEN);
        assert_eq!(err.code, "step_up_required");
    }
}
