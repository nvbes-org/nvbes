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
    amr: &[&str],
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
        amr.iter().map(|method| (*method).to_string()).collect(),
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
async fn email_mutation_rejects_aal2_password_session() {
    let Some(redis) = crate::test_support::test_redis_pool().await else {
        eprintln!("skipping test: redis not available");
        return;
    };
    let subject = TestStepUpSubject {
        principal_id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
    };
    store_session(
        &redis,
        &subject,
        "aal2",
        &["pwd"],
        Some(Utc::now() + chrono::Duration::minutes(10)),
    )
    .await;

    let error = require_email_mutation_step_up(&redis, &subject)
        .await
        .expect_err("AAL2 password authentication must not authorize an email mutation");

    assert_eq!(error.status, axum::http::StatusCode::FORBIDDEN);
    assert_eq!(error.code, "phishing_resistant_step_up_required");
    let _ =
        nvbes_redis::session::clear_user_sessions(&redis, &subject.principal_id.to_string()).await;
}

#[tokio::test]
async fn email_mutation_rejects_aal2_totp_session() {
    let Some(redis) = crate::test_support::test_redis_pool().await else {
        eprintln!("skipping test: redis not available");
        return;
    };
    let subject = TestStepUpSubject {
        principal_id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
    };
    store_session(
        &redis,
        &subject,
        "aal2",
        &["otp"],
        Some(Utc::now() + chrono::Duration::minutes(10)),
    )
    .await;

    let error = require_email_mutation_step_up(&redis, &subject)
        .await
        .expect_err("AAL2 TOTP authentication must not authorize an email mutation");

    assert_eq!(error.status, axum::http::StatusCode::FORBIDDEN);
    assert_eq!(error.code, "phishing_resistant_step_up_required");
    let _ =
        nvbes_redis::session::clear_user_sessions(&redis, &subject.principal_id.to_string()).await;
}

#[tokio::test]
async fn email_mutation_accepts_recent_webauthn_session() {
    let Some(redis) = crate::test_support::test_redis_pool().await else {
        eprintln!("skipping test: redis not available");
        return;
    };
    let subject = TestStepUpSubject {
        principal_id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
    };
    store_session(
        &redis,
        &subject,
        "aal2",
        &["webauthn"],
        Some(Utc::now() + chrono::Duration::minutes(10)),
    )
    .await;

    require_email_mutation_step_up(&redis, &subject)
        .await
        .expect("recent WebAuthn authentication should authorize an email mutation");

    let _ =
        nvbes_redis::session::clear_user_sessions(&redis, &subject.principal_id.to_string()).await;
}

#[tokio::test]
async fn email_mutation_rejects_expired_webauthn_session() {
    let Some(redis) = crate::test_support::test_redis_pool().await else {
        eprintln!("skipping test: redis not available");
        return;
    };
    let subject = TestStepUpSubject {
        principal_id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
    };
    store_session(
        &redis,
        &subject,
        "aal2",
        &["webauthn"],
        Some(Utc::now() - chrono::Duration::seconds(1)),
    )
    .await;

    let error = require_email_mutation_step_up(&redis, &subject)
        .await
        .expect_err("expired WebAuthn authentication must not authorize an email mutation");

    assert_eq!(error.status, axum::http::StatusCode::FORBIDDEN);
    assert_eq!(error.code, "phishing_resistant_step_up_required");
    let _ =
        nvbes_redis::session::clear_user_sessions(&redis, &subject.principal_id.to_string()).await;
}
