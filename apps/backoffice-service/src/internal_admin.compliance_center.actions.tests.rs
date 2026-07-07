use axum::{Router, body, http::StatusCode};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

use crate::compliance_center_actions_test_support::{
    compliance_action_count, compliance_actions_schema_exists, compliance_revoke_audit_count,
    idempotency_schema_exists, revoke_consent_request, revoke_consent_request_with_key,
    revoke_consent_request_without_second_approver, seed_workspace_actor_principal_and_consent,
    test_pool,
};

#[tokio::test]
async fn revoke_consent_route_enforces_grant_confirmation_dual_control_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !compliance_actions_schema_exists(&pool).await {
        eprintln!("skipping test: compliance action schema is missing");
        return;
    }
    if !crate::test_operator_grants::operator_grants_schema_exists(&pool).await {
        eprintln!("skipping test: operator grant schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let (tenant_id, workspace_id, consent_id) =
        seed_workspace_actor_principal_and_consent(&pool, actor_id, principal_id).await;
    let strong_code =
        crate::backoffice_authorization::strong_confirmation_code("REVOKE CONSENT", consent_id);
    let app = Router::new()
        .merge(crate::compliance_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let denied = app
        .clone()
        .oneshot(revoke_consent_request(
            workspace_id,
            actor_id,
            "viewer",
            consent_id,
            &strong_code,
            "ticket GDPR-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    for role in ["finance_admin", "operations_admin", "security_admin"] {
        let denied = app
            .clone()
            .oneshot(revoke_consent_request(
                workspace_id,
                actor_id,
                role,
                consent_id,
                &strong_code,
                "ticket GDPR-123 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN, "{role}");
    }

    let wrong_confirmation = app
        .clone()
        .oneshot(revoke_consent_request(
            workspace_id,
            actor_id,
            "compliance_admin",
            consent_id,
            "REVOKE",
            "ticket GDPR-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

    let generic_confirmation = app
        .clone()
        .oneshot(revoke_consent_request(
            workspace_id,
            actor_id,
            "compliance_admin",
            consent_id,
            "REVOKE CONSENT",
            "ticket GDPR-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(generic_confirmation.status(), StatusCode::BAD_REQUEST);

    let missing_dual_control = app
        .clone()
        .oneshot(revoke_consent_request_without_second_approver(
            workspace_id,
            actor_id,
            "compliance_admin",
            consent_id,
            &strong_code,
            "ticket GDPR-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_dual_control.status(), StatusCode::BAD_REQUEST);

    let missing_grant = app
        .clone()
        .oneshot(revoke_consent_request(
            workspace_id,
            actor_id,
            "compliance_admin",
            consent_id,
            &strong_code,
            "ticket GDPR-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_grant.status(), StatusCode::FORBIDDEN);

    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "compliance_admin")
        .await;

    let accepted = app
        .oneshot(revoke_consent_request(
            workspace_id,
            actor_id,
            "compliance_admin",
            consent_id,
            &strong_code,
            "ticket GDPR-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = body::to_bytes(accepted.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("body should be json");
    assert_eq!(payload["action_kind"], json!("revoke_consent"));
    assert_eq!(payload["status"], json!("applied"));

    let revoked = sqlx::query_scalar::<_, bool>(
        "SELECT revoked_at IS NOT NULL FROM user_consents WHERE id = $1",
    )
    .bind(consent_id)
    .fetch_one(&pool)
    .await
    .expect("consent status should load");
    assert!(revoked);
    assert_eq!(
        compliance_action_count(&pool, tenant_id, actor_id, consent_id).await,
        1
    );
    assert_eq!(
        compliance_revoke_audit_count(&pool, tenant_id, actor_id).await,
        1
    );

    let audit_metadata = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT metadata FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'compliance.consent.revoked'
           AND target_type = 'user_consent'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&pool)
    .await
    .expect("audit metadata should load");
    assert_eq!(audit_metadata["compliance_action_id"], payload["object_id"]);
    assert_eq!(
        audit_metadata["object_links"]["consent_id"],
        json!(consent_id)
    );
    assert_eq!(
        audit_metadata["changes"][0],
        json!({
            "field": "revoked_at",
            "before": null,
            "after": "recorded"
        })
    );
}

#[tokio::test]
async fn revoke_consent_reuses_stored_idempotent_response() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !compliance_actions_schema_exists(&pool).await {
        eprintln!("skipping test: compliance action schema is missing");
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
    let (tenant_id, workspace_id, consent_id) =
        seed_workspace_actor_principal_and_consent(&pool, actor_id, principal_id).await;
    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "compliance_admin")
        .await;

    let state = crate::app::AppState::new(nvbes_core::config::AppConfig::default(), pool.clone());
    let app = Router::new()
        .merge(crate::compliance_center_actions::router())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::idempotency::idempotency_guard,
        ))
        .with_state(state);
    let confirm_code =
        crate::backoffice_authorization::strong_confirmation_code("REVOKE CONSENT", consent_id);
    let idempotency_key = format!("test-{}", Uuid::new_v4());

    let first = app
        .clone()
        .oneshot(revoke_consent_request_with_key(
            workspace_id,
            actor_id,
            "compliance_admin",
            consent_id,
            &confirm_code,
            "ticket GDPR-123 approved",
            &idempotency_key,
        ))
        .await
        .expect("route should respond");
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .oneshot(revoke_consent_request_with_key(
            workspace_id,
            actor_id,
            "compliance_admin",
            consent_id,
            &confirm_code,
            "ticket GDPR-123 approved",
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

    let revoked = sqlx::query_scalar::<_, bool>(
        "SELECT revoked_at IS NOT NULL FROM user_consents WHERE id = $1",
    )
    .bind(consent_id)
    .fetch_one(&pool)
    .await
    .expect("consent status should load");
    assert!(revoked);
    assert_eq!(
        compliance_action_count(&pool, tenant_id, actor_id, consent_id).await,
        1
    );
    assert_eq!(
        compliance_revoke_audit_count(&pool, tenant_id, actor_id).await,
        1
    );
}
