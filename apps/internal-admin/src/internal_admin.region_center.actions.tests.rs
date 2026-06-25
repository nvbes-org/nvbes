use axum::{Router, body, http::StatusCode};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

use crate::region_center_actions_test_support::{
    flag_request, flag_request_with_key, flag_request_without_second_approver,
    idempotency_schema_exists, region_action_count, region_actions_schema_exists,
    region_audit_count, seed_workspace_with_actor, test_pool,
};

#[tokio::test]
async fn flag_residency_route_enforces_grant_confirmation_dual_control_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !region_actions_schema_exists(&pool).await {
        eprintln!("skipping test: region action schema is missing");
        return;
    }
    if !crate::test_operator_grants::operator_grants_schema_exists(&pool).await {
        eprintln!("skipping test: operator grant schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let (tenant_id, workspace_id) = seed_workspace_with_actor(&pool, actor_id).await;
    let strong_code =
        crate::backoffice_authorization::strong_confirmation_code("FLAG RESIDENCY", workspace_id);
    let app = Router::new()
        .merge(crate::region_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let denied = app
        .clone()
        .oneshot(flag_request(
            workspace_id,
            actor_id,
            "viewer",
            &strong_code,
            "ticket REG-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);

    for role in ["finance_admin", "operations_admin", "security_admin"] {
        let denied = app
            .clone()
            .oneshot(flag_request(
                workspace_id,
                actor_id,
                role,
                &strong_code,
                "ticket REG-123 approved",
            ))
            .await
            .expect("route should respond");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN, "{role}");
    }

    let wrong_confirmation = app
        .clone()
        .oneshot(flag_request(
            workspace_id,
            actor_id,
            "compliance_admin",
            "FLAG",
            "ticket REG-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(wrong_confirmation.status(), StatusCode::BAD_REQUEST);

    let generic_confirmation = app
        .clone()
        .oneshot(flag_request(
            workspace_id,
            actor_id,
            "compliance_admin",
            "FLAG RESIDENCY",
            "ticket REG-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(generic_confirmation.status(), StatusCode::BAD_REQUEST);

    let missing_dual_control = app
        .clone()
        .oneshot(flag_request_without_second_approver(
            workspace_id,
            actor_id,
            "compliance_admin",
            &strong_code,
            "ticket REG-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_dual_control.status(), StatusCode::BAD_REQUEST);

    let missing_grant = app
        .clone()
        .oneshot(flag_request(
            workspace_id,
            actor_id,
            "compliance_admin",
            &strong_code,
            "ticket REG-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_grant.status(), StatusCode::FORBIDDEN);

    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "compliance_admin")
        .await;

    let accepted = app
        .oneshot(flag_request(
            workspace_id,
            actor_id,
            "compliance_admin",
            &strong_code,
            "ticket REG-123 approved",
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = body::to_bytes(accepted.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("body should be json");
    assert_eq!(payload["action_kind"], json!("flag_residency"));
    assert_eq!(payload["status"], json!("applied"));

    let region =
        sqlx::query_scalar::<_, String>("SELECT data_region::text FROM workspaces WHERE id = $1")
            .bind(workspace_id)
            .fetch_one(&pool)
            .await
            .expect("workspace region should load");
    assert_eq!(region, "us");
    assert_eq!(
        region_action_count(&pool, tenant_id, actor_id, workspace_id).await,
        1
    );
    assert_eq!(region_audit_count(&pool, tenant_id, actor_id).await, 1);

    let audit_metadata = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT metadata FROM audit_events
         WHERE tenant_id = $1 AND actor_principal_id = $2
           AND action = 'region.residency.flagged'
           AND target_type = 'workspace'",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .fetch_one(&pool)
    .await
    .expect("audit metadata should load");
    assert_eq!(audit_metadata["region_action_id"], payload["object_id"]);
    assert_eq!(
        audit_metadata["object_links"]["workspace_id"],
        json!(workspace_id)
    );
    assert_eq!(
        audit_metadata["changes"][0],
        json!({
            "field": "data_region",
            "before": "eu",
            "after": "us"
        })
    );
}

#[tokio::test]
async fn flag_residency_reuses_stored_idempotent_response() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !region_actions_schema_exists(&pool).await {
        eprintln!("skipping test: region action schema is missing");
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
    let (tenant_id, workspace_id) = seed_workspace_with_actor(&pool, actor_id).await;
    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "compliance_admin")
        .await;

    let state = crate::app::AppState::new(nvbes_core::config::AppConfig::default(), pool.clone());
    let app = Router::new()
        .merge(crate::region_center_actions::router())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::idempotency::idempotency_guard,
        ))
        .with_state(state);
    let confirm_code =
        crate::backoffice_authorization::strong_confirmation_code("FLAG RESIDENCY", workspace_id);
    let idempotency_key = format!("test-{}", Uuid::new_v4());

    let first = app
        .clone()
        .oneshot(flag_request_with_key(
            workspace_id,
            actor_id,
            "compliance_admin",
            &confirm_code,
            "ticket REG-123 approved",
            &idempotency_key,
        ))
        .await
        .expect("route should respond");
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .oneshot(flag_request_with_key(
            workspace_id,
            actor_id,
            "compliance_admin",
            &confirm_code,
            "ticket REG-123 approved",
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

    let region =
        sqlx::query_scalar::<_, String>("SELECT data_region::text FROM workspaces WHERE id = $1")
            .bind(workspace_id)
            .fetch_one(&pool)
            .await
            .expect("workspace region should load");
    assert_eq!(region, "us");
    assert_eq!(
        region_action_count(&pool, tenant_id, actor_id, workspace_id).await,
        1
    );
    assert_eq!(region_audit_count(&pool, tenant_id, actor_id).await, 1);
}
