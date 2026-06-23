#[path = "drive.domains.public_api.db_tests.support.rs"]
mod support;

use sqlx::Row;
use uuid::Uuid;

use super::{
    network_policy::public_api_network_block,
    observability::{log_denied, log_request, record_api_audit_event},
    types::{DeniedLogInput, PublicApiAuditEventInput, PublicApiLogInput},
};
use support::{
    context, db_supports_public_api_geo_schema, seed_vpn_range, seed_workspace, test_pool,
};

#[tokio::test]
async fn public_api_request_log_persists_geo_labels() {
    let pool = test_pool();
    if !db_supports_public_api_geo_schema(&pool).await {
        eprintln!("skipping test: local database is missing current public API geo schema");
        return;
    }
    let key = format!("public-api-log-{}", Uuid::new_v4());
    let (principal_id, workspace_id, tenant_id) = seed_workspace(&pool, &key).await;
    seed_vpn_range(&pool, &key).await;
    let request_id = format!("{key}-request");

    log_request(
        &pool,
        &context(workspace_id, principal_id, tenant_id, request_id.clone()),
        PublicApiLogInput {
            method: "GET",
            path: "/v1/files",
            status_code: 200,
            error_code: None,
            scopes_used: &["files:read"],
            ip: Some("8.8.4.42"),
            user_agent: Some("public-api-test"),
        },
    )
    .await
    .expect("request log should be persisted");

    let row = sqlx::query(
        r#"
        SELECT geo_country_code, geo_source, geo_confidence, geo_network_kind, geo_risk_score, geo_risk_labels
        FROM api_request_logs
        WHERE workspace_id = $1 AND request_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(&request_id)
    .fetch_one(&pool)
    .await
    .expect("request log row should exist");

    assert_eq!(
        row.get::<Option<String>, _>("geo_country_code").as_deref(),
        Some("FR")
    );
    assert_eq!(
        row.get::<Option<String>, _>("geo_network_kind").as_deref(),
        Some("vpn")
    );
    assert_eq!(row.get::<Option<i16>, _>("geo_risk_score"), Some(95));
    assert!(
        row.get::<Vec<String>, _>("geo_risk_labels")
            .contains(&"vpn".to_string())
    );
}

#[tokio::test]
async fn public_api_audit_denied_and_network_policy_are_geo_labeled() {
    let pool = test_pool();
    if !db_supports_public_api_geo_schema(&pool).await {
        eprintln!("skipping test: local database is missing current public API geo schema");
        return;
    }
    let key = format!("public-api-audit-{}", Uuid::new_v4());
    let (principal_id, workspace_id, tenant_id) = seed_workspace(&pool, &key).await;
    seed_vpn_range(&pool, &key).await;

    let block = public_api_network_block(&pool, workspace_id, Some("8.8.4.42"))
        .await
        .expect("network policy should resolve")
        .expect("vpn range should be blocked");
    assert_eq!(block.reason, "vpn");

    let request_id = format!("{key}-denied");
    log_denied(
        &pool,
        DeniedLogInput {
            workspace_id,
            api_key_id: None,
            actor_principal_id: Some(principal_id),
            request_id: &request_id,
            error_code: "network_risk_blocked",
            network_block_reason: Some(block.reason),
            ip: Some("8.8.4.42"),
            user_agent: Some("public-api-test"),
            scopes_used: &["files:read"],
        },
    )
    .await
    .expect("denied log should be persisted");

    let audit_row = sqlx::query(
        r#"
        SELECT metadata
        FROM audit_events
        WHERE workspace_id = $1
          AND action = 'api.request.denied'
          AND metadata->>'request_id' = $2
        "#,
    )
    .bind(workspace_id)
    .bind(&request_id)
    .fetch_one(&pool)
    .await
    .expect("denied audit row should exist");
    let metadata: sqlx::types::Json<serde_json::Value> = audit_row.get("metadata");
    assert_eq!(metadata["network_block_reason"], "vpn");
    assert_eq!(metadata["geo"]["geo_network_kind"], "vpn");
    assert_eq!(metadata["geo"]["geo_risk_score"], 95);

    record_api_audit_event(
        &pool,
        &context(
            workspace_id,
            principal_id,
            tenant_id,
            format!("{key}-audit"),
        ),
        PublicApiAuditEventInput {
            action: "api.file.read",
            target_type: "file",
            target_id: None,
            ip: Some("8.8.4.42"),
            user_agent: Some("public-api-test"),
            metadata: serde_json::json!({"surface": "public_api"}),
        },
    )
    .await
    .expect("public API audit event should be persisted");

    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM audit_events
        WHERE workspace_id = $1
          AND metadata #>> '{geo,geo_network_kind}' = 'vpn'
          AND EXISTS (
            SELECT 1
            FROM jsonb_array_elements_text(metadata #> '{geo,geo_risk_labels}') AS label(value)
            WHERE label.value = 'vpn'
          )
        "#,
    )
    .bind(workspace_id)
    .fetch_one(&pool)
    .await
    .expect("audit count should succeed");
    assert!(count >= 2);
}

#[tokio::test]
async fn public_api_network_policy_respects_disabled_mode_and_allowlist() {
    let pool = test_pool();
    if !db_supports_public_api_geo_schema(&pool).await {
        eprintln!("skipping test: local database is missing current public API geo schema");
        return;
    }
    let key = format!("public-api-policy-{}", Uuid::new_v4());
    let (_principal_id, workspace_id, _tenant_id) = seed_workspace(&pool, &key).await;
    seed_vpn_range(&pool, &key).await;

    sqlx::query(
        "UPDATE workspace_policies SET public_api_network_policy_mode = 'disabled' WHERE workspace_id = $1",
    )
    .bind(workspace_id)
    .execute(&pool)
    .await
    .expect("policy update should succeed");

    let disabled_block = public_api_network_block(&pool, workspace_id, Some("8.8.4.42"))
        .await
        .expect("disabled policy should resolve");
    assert!(disabled_block.is_none());

    sqlx::query(
        r#"
        UPDATE workspace_policies
        SET public_api_network_policy_mode = 'enforce'
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .execute(&pool)
    .await
    .expect("policy update should succeed");
    sqlx::query(
        r#"
        INSERT INTO public_api_network_allowlist (workspace_id, cidr, reason)
        VALUES ($1, '8.8.4.0/24', 'integration test allowlist')
        "#,
    )
    .bind(workspace_id)
    .execute(&pool)
    .await
    .expect("allowlist insert should succeed");

    let allowlisted_block = public_api_network_block(&pool, workspace_id, Some("8.8.4.42"))
        .await
        .expect("allowlisted policy should resolve");
    assert!(allowlisted_block.is_none());
}
