use axum::{Router, body, http::StatusCode};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

use crate::access_center_actions_test_support::{
    access_schema_exists, idempotency_schema_exists, seed_workspace_membership, suspend_request,
    suspend_request_with_key, test_pool,
};

#[tokio::test]
async fn suspend_membership_rejects_generic_confirmation_and_audits_success() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !access_schema_exists(&pool).await {
        eprintln!("skipping test: access schema is missing");
        return;
    }
    if !crate::test_operator_grants::operator_grants_schema_exists(&pool).await {
        eprintln!("skipping test: operator grant schema is missing");
        return;
    }

    let actor_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let (tenant_id, workspace_id) =
        seed_workspace_membership(&pool, actor_id, target_id, owner_id).await;
    let app = Router::new()
        .merge(crate::access_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool.clone(),
        ));

    let generic_confirmation = app
        .clone()
        .oneshot(suspend_request(
            workspace_id,
            target_id,
            actor_id,
            "security_admin",
            "SUSPEND ACCESS",
        ))
        .await
        .expect("route should respond");
    assert_eq!(generic_confirmation.status(), StatusCode::BAD_REQUEST);

    let missing_grant = app
        .clone()
        .oneshot(suspend_request(
            workspace_id,
            target_id,
            actor_id,
            "security_admin",
            &crate::backoffice_authorization::strong_confirmation_code("SUSPEND ACCESS", target_id),
        ))
        .await
        .expect("route should respond");
    assert_eq!(missing_grant.status(), StatusCode::FORBIDDEN);

    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "security_admin")
        .await;

    let accepted = app
        .oneshot(suspend_request(
            workspace_id,
            target_id,
            actor_id,
            "security_admin",
            &crate::backoffice_authorization::strong_confirmation_code("SUSPEND ACCESS", target_id),
        ))
        .await
        .expect("route should respond");
    assert_eq!(accepted.status(), StatusCode::OK);
    let body = body::to_bytes(accepted.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("body should be json");
    assert_eq!(payload["principal_id"], json!(target_id.to_string()));
    assert_eq!(payload["next_status"], json!("suspended"));

    let status = sqlx::query_scalar::<_, String>(
        "SELECT status::text FROM workspace_memberships
         WHERE workspace_id = $1 AND principal_id = $2",
    )
    .bind(workspace_id)
    .bind(target_id)
    .fetch_one(&pool)
    .await
    .expect("membership status should load");
    assert_eq!(status, "suspended");

    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND workspace_id = $2 AND actor_principal_id = $3
           AND action = 'internal_admin.access.workspace_membership.suspended'
           AND target_type = 'workspace_membership'
           AND target_id = $4",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(actor_id)
    .bind(target_id)
    .fetch_one(&pool)
    .await
    .expect("audit count should load");
    assert_eq!(audit_count, 1);
}

#[tokio::test]
async fn suspend_membership_reuses_stored_idempotent_response() {
    let Some(pool) = test_pool().await else {
        eprintln!("skipping test: Postgres is not reachable");
        return;
    };
    if !access_schema_exists(&pool).await {
        eprintln!("skipping test: access schema is missing");
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
    let target_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let (tenant_id, workspace_id) =
        seed_workspace_membership(&pool, actor_id, target_id, owner_id).await;
    crate::test_operator_grants::grant_active_operator_role(&pool, actor_id, "security_admin")
        .await;

    let state = crate::app::AppState::new(nvbes_core::config::AppConfig::default(), pool.clone());
    let app = Router::new()
        .merge(crate::access_center_actions::router())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::idempotency::idempotency_guard,
        ))
        .with_state(state);
    let confirm_code =
        crate::backoffice_authorization::strong_confirmation_code("SUSPEND ACCESS", target_id);
    let idempotency_key = format!("test-{}", Uuid::new_v4());

    let first = app
        .clone()
        .oneshot(suspend_request_with_key(
            workspace_id,
            target_id,
            actor_id,
            "security_admin",
            &confirm_code,
            &idempotency_key,
        ))
        .await
        .expect("route should respond");
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .oneshot(suspend_request_with_key(
            workspace_id,
            target_id,
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

    let audit_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM audit_events
         WHERE tenant_id = $1 AND workspace_id = $2 AND actor_principal_id = $3
           AND action = 'internal_admin.access.workspace_membership.suspended'
           AND target_type = 'workspace_membership'
           AND target_id = $4",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(actor_id)
    .bind(target_id)
    .fetch_one(&pool)
    .await
    .expect("audit count should load");
    assert_eq!(audit_count, 1);

    let status = sqlx::query_scalar::<_, String>(
        "SELECT status::text FROM workspace_memberships
         WHERE workspace_id = $1 AND principal_id = $2",
    )
    .bind(workspace_id)
    .bind(target_id)
    .fetch_one(&pool)
    .await
    .expect("membership status should load");
    assert_eq!(status, "suspended");
}
