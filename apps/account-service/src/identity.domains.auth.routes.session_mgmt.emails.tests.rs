use chrono::Utc;
use uuid::Uuid;

use super::require_email_mutation_step_up;
use crate::domains::auth::sessions::cache::{cached_session_from_login, current_session_ttl};
use crate::domains::auth::types::StepUpSubject;

struct TestStepUpSubject {
    principal_id: Uuid,
    session_id: Uuid,
}

impl StepUpSubject for TestStepUpSubject {
    fn user_id(&self) -> Uuid {
        self.principal_id
    }

    fn session_id(&self) -> Uuid {
        self.session_id
    }

    fn tenant_id(&self) -> Option<Uuid> {
        None
    }

    fn workspace_id(&self) -> Option<Uuid> {
        None
    }
}

async fn store_session(
    redis: &nvbes_redis::RedisPool,
    subject: &TestStepUpSubject,
    aal: &str,
    step_up_expires_at: Option<chrono::DateTime<Utc>>,
) {
    let now = Utc::now();
    let mut session = cached_session_from_login(
        subject.session_id,
        subject.principal_id,
        None,
        None,
        None,
        None,
        None,
        format!("session-token-{}", subject.session_id),
        Some(aal.to_string()),
        vec!["pwd".to_string()],
        now,
        now,
        None,
        None,
        now + chrono::Duration::hours(2),
    );
    session.step_up_verified_at = step_up_expires_at.map(|_| now);
    session.step_up_expires_at = step_up_expires_at;
    nvbes_redis::session::set_session(redis, &session, current_session_ttl(&session))
        .await
        .expect("test session should be stored");
}

#[tokio::test]
async fn email_mutation_rejects_aal1_session() {
    let redis = crate::test_support::test_redis_pool().await;
    let subject = TestStepUpSubject {
        principal_id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
    };
    store_session(
        &redis,
        &subject,
        "aal1",
        Some(Utc::now() + chrono::Duration::minutes(10)),
    )
    .await;

    let error = require_email_mutation_step_up(&redis, &subject)
        .await
        .expect_err("AAL1 must not authorize an email mutation");

    assert_eq!(error.status, axum::http::StatusCode::UNAUTHORIZED);
    assert_eq!(error.code, "step_up_required");
    let _ =
        nvbes_redis::session::clear_user_sessions(&redis, &subject.principal_id.to_string()).await;
}

#[tokio::test]
async fn email_mutation_accepts_recent_aal2_session() {
    let redis = crate::test_support::test_redis_pool().await;
    let subject = TestStepUpSubject {
        principal_id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
    };
    store_session(
        &redis,
        &subject,
        "aal2",
        Some(Utc::now() + chrono::Duration::minutes(10)),
    )
    .await;

    require_email_mutation_step_up(&redis, &subject)
        .await
        .expect("recent AAL2 should authorize an email mutation");

    let _ =
        nvbes_redis::session::clear_user_sessions(&redis, &subject.principal_id.to_string()).await;
}

#[tokio::test]
async fn email_mutation_rejects_expired_aal2_session() {
    let redis = crate::test_support::test_redis_pool().await;
    let subject = TestStepUpSubject {
        principal_id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
    };
    store_session(
        &redis,
        &subject,
        "aal2",
        Some(Utc::now() - chrono::Duration::seconds(1)),
    )
    .await;

    let error = require_email_mutation_step_up(&redis, &subject)
        .await
        .expect_err("expired AAL2 must not authorize an email mutation");

    assert_eq!(error.status, axum::http::StatusCode::UNAUTHORIZED);
    assert_eq!(error.code, "step_up_required");
    let _ =
        nvbes_redis::session::clear_user_sessions(&redis, &subject.principal_id.to_string()).await;
}
