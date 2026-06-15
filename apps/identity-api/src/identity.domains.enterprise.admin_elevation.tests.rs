use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use super::*;
use crate::domains::auth::sessions::cache::{cached_session_from_login, current_session_ttl};
use crate::http::middleware::jwt::AuthContext;

#[test]
fn elevation_expiry_never_outlives_step_up_or_session() {
    let now = Utc::now();
    let requested = now + Duration::minutes(60);
    let step_up = now + Duration::minutes(15);
    let session = now + Duration::minutes(30);

    assert_eq!(
        bounded_elevation_expires_at(requested, step_up, session),
        step_up
    );
}

#[test]
fn elevation_expiry_uses_session_when_session_expires_first() {
    let now = Utc::now();
    let requested = now + Duration::minutes(60);
    let step_up = now + Duration::minutes(30);
    let session = now + Duration::minutes(10);

    assert_eq!(
        bounded_elevation_expires_at(requested, step_up, session),
        session
    );
}

#[tokio::test]
async fn grant_rejects_non_admin_base_role() {
    let redis = crate::test_support::test_redis_pool().await;
    let auth = test_auth_context(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());

    let err = grant_admin_elevation(
        &redis,
        &auth,
        &EnterpriseRole::Member,
        auth.tenant_id.expect("tenant id should be present"),
        EnterpriseAdminElevationInput {
            duration_minutes: None,
        },
    )
    .await
    .expect_err("members must not self-elevate");

    assert_eq!(err.code, "admin_role_required");
}

#[tokio::test]
async fn grant_requires_recent_step_up() {
    let redis = crate::test_support::test_redis_pool().await;
    let principal_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let auth = test_auth_context(principal_id, session_id, tenant_id);
    seed_session(&redis, principal_id, session_id, tenant_id, None).await;

    let err = grant_admin_elevation(
        &redis,
        &auth,
        &EnterpriseRole::Admin,
        tenant_id,
        EnterpriseAdminElevationInput {
            duration_minutes: None,
        },
    )
    .await
    .expect_err("step-up is mandatory before elevation");

    assert_eq!(err.code, "step_up_required");
    cleanup_session(&redis, principal_id).await;
}

#[tokio::test]
async fn grant_stores_elevation_bounded_by_step_up() {
    let redis = crate::test_support::test_redis_pool().await;
    let principal_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let auth = test_auth_context(principal_id, session_id, tenant_id);
    let step_up_expires_at = Utc::now() + Duration::minutes(10);
    seed_session(
        &redis,
        principal_id,
        session_id,
        tenant_id,
        Some(step_up_expires_at),
    )
    .await;

    let response = grant_admin_elevation(
        &redis,
        &auth,
        &EnterpriseRole::Admin,
        tenant_id,
        EnterpriseAdminElevationInput {
            duration_minutes: Some(60),
        },
    )
    .await
    .expect("active admins with step-up can elevate");

    let elevation = active_admin_elevation(&redis, &auth, tenant_id)
        .await
        .expect("elevation lookup should succeed")
        .expect("elevation should be active");
    assert!(response.elevation.active);
    assert!(elevation.active);
    assert!(elevation.expires_at.expect("expiry") <= step_up_expires_at);
    cleanup_session(&redis, principal_id).await;
}

fn test_auth_context(principal_id: Uuid, session_id: Uuid, tenant_id: Uuid) -> AuthContext {
    AuthContext {
        user_id: principal_id,
        user_email: "admin@example.com".to_string(),
        display_name: "Admin Tester".to_string(),
        email_verified_at: Some(Utc::now()),
        mfa_enabled: true,
        tenant_id: Some(tenant_id),
        organization_id: None,
        workspace_id: None,
        workspace_region: None,
        token_type: "access".to_string(),
        scope: "openid profile email".to_string(),
        jti: session_id.to_string(),
        session_id,
        acr: Some("aal2".to_string()),
        amr: vec!["pwd".to_string(), "otp".to_string()],
        auth_time: Some(Utc::now().timestamp()),
        client_id: None,
        cnf_jkt: None,
    }
}

async fn seed_session(
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
    session_id: Uuid,
    tenant_id: Uuid,
    step_up_expires_at: Option<DateTime<Utc>>,
) {
    let now = Utc::now();
    let mut session = cached_session_from_login(
        session_id,
        principal_id,
        Some(tenant_id),
        None,
        None,
        None,
        None,
        "admin-elevation-test-token".to_string(),
        Some("aal2".to_string()),
        vec!["pwd".to_string(), "otp".to_string()],
        now,
        now,
        None,
        None,
        now + Duration::minutes(30),
    );
    session.step_up_verified_at = step_up_expires_at.map(|_| now);
    session.step_up_expires_at = step_up_expires_at;
    nvbes_redis::session::set_session(redis, &session, current_session_ttl(&session))
        .await
        .expect("session should be cached");
}

async fn cleanup_session(redis: &nvbes_redis::RedisPool, principal_id: Uuid) {
    let _ = nvbes_redis::session::clear_user_sessions(redis, &principal_id.to_string()).await;
}
