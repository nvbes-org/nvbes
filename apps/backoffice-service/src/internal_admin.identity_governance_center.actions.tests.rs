use axum::{Router, body, http::StatusCode};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

use crate::identity_governance_center_actions_test_support::{
    break_glass_audit_count, break_glass_audit_metadata, break_glass_revoked,
    governance_schema_exists, idempotency_schema_exists, revoke_break_glass_request,
    revoke_break_glass_request_with_key, revoke_break_glass_request_without_second_approver,
    seed_break_glass_account, test_pool,
};

#[tokio::test]
async fn revoke_break_glass_enforces_grant_confirmation_dual_control_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !governance_schema_exists(&pool).await {
        eprintln!("skipping test: governance schema is missing");
        return;
    }
    if !crate::test_operator_grants::operator_grants_schema_exists(&pool).await {
        eprintln!("skipping test: operator grant schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let tenant_id = seed_break_glass_account(&pool, actor_id, principal_id).await;
    let strong_code = crate::backoffice_authorization::strong_confirmation_code(
        "REVOKE BREAK GLASS",
        principal_id,
    );
    let app = Router::new()
        .merge(crate::identity_governance_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let denied = app
        .clone()
        .oneshot(revoke_break_glass_request(
            tenant_id,
            principal_id,
            actor_id,
            "viewer",
            &strong_code,
        ))
        .await
        .expect("route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    for role in ["finance_admin", "operations_admin", "compliance_admin"] {
        let denied = app
            .clone()
            .oneshot(revoke_break_glass_request(
                tenant_id,
                principal_id,
                actor_id,
                role,
                &strong_code,
            ))
            .await
            .expect("route should respond");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN, "{role}");
    }

    let generic_confirmation = app
        .clone()
        .oneshot(revoke_break_glass_request(
            tenant_id,
            principal_id,
            actor_id,
            "security_admin",
            "REVOKE BREAK GLASS",
        ))
        .await
        .expect("route should respond");
    assert_eq!(generic_confirmation.status(), StatusCode::BAD_REQUEST);

    let missing_dual_control = app
        .clone()
        .oneshot(revoke_break_glass_request_without_second_approver(
            tenant_id,
            principal_id,
            actor_id,
            "security_admin",
            &strong_code,
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_dual_control.status(), StatusCode::BAD_REQUEST);

    let missing_grant = app
        .clone()
        .oneshot(revoke_break_glass_request(
            tenant_id,
            principal_id,
            actor_id,
            "security_admin",
            &strong_code,
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_grant.status(), StatusCode::FORBIDDEN);

    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "security_admin")
        .await;

    let accepted = app
        .oneshot(revoke_break_glass_request(
            tenant_id,
            principal_id,
            actor_id,
            "security_admin",
            &strong_code,
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = body::to_bytes(accepted.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("body should be json");
    assert_eq!(payload["object_id"], json!(principal_id.to_string()));
    assert_eq!(
        payload["audit_action"],
        json!("internal_admin.identity_governance.break_glass.revoked")
    );
    assert!(break_glass_revoked(&pool, tenant_id, principal_id).await);
    assert_eq!(
        break_glass_audit_count(&pool, tenant_id, actor_id, principal_id).await,
        1
    );
    let metadata = break_glass_audit_metadata(&pool, tenant_id, actor_id, principal_id).await;
    assert_eq!(
        metadata["object_links"]["principal_id"],
        json!(principal_id.to_string())
    );
    assert_eq!(
        metadata["changes"][0],
        json!({
            "field": "break_glass.revoked_at",
            "before": "active",
            "after": "revoked"
        })
    );
}

#[tokio::test]
async fn revoke_break_glass_reuses_stored_idempotent_response() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !governance_schema_exists(&pool).await {
        eprintln!("skipping test: governance schema is missing");
        return;
    }
    if !crate::test_operator_grants::operator_grants_schema_exists(&pool).await {
        eprintln!("skipping test: operator grant schema is missing");
        return;
    }
    if !idempotency_schema_exists(&pool).await {
        eprintln!("skipping test: idempotency response schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let tenant_id = seed_break_glass_account(&pool, actor_id, principal_id).await;
    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "security_admin")
        .await;

    let state = crate::app::AppState::new(nvbes_core::config::AppConfig::default(), pool.clone());
    let app = Router::new()
        .merge(crate::identity_governance_center_actions::router())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::idempotency::idempotency_guard,
        ))
        .with_state(state);
    let confirm_code = crate::backoffice_authorization::strong_confirmation_code(
        "REVOKE BREAK GLASS",
        principal_id,
    );
    let idempotency_key = format!("test-{}", Uuid::new_v4());

    let first = app
        .clone()
        .oneshot(revoke_break_glass_request_with_key(
            tenant_id,
            principal_id,
            actor_id,
            "security_admin",
            &confirm_code,
            &idempotency_key,
        ))
        .await
        .expect("route should respond");
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .oneshot(revoke_break_glass_request_with_key(
            tenant_id,
            principal_id,
            actor_id,
            "security_admin",
            &confirm_code,
            &idempotency_key,
        ))
        .await
        .expect("route should respond");
    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(
        second
            .headers()
            .get("idempotency-replayed")
            .and_then(|value| value.to_str().ok()),
        Some("true")
    );

    assert!(break_glass_revoked(&pool, tenant_id, principal_id).await);
    assert_eq!(
        break_glass_audit_count(&pool, tenant_id, actor_id, principal_id).await,
        1
    );
}
