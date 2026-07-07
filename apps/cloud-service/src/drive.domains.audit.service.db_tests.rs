use sqlx::Row;
use uuid::Uuid;

use super::{AuditRecordInput, ListAuditEventsInput, list_events, record_event};
use crate::domains::quotas::{BandwidthOutUsageInput, record_bandwidth_out_tx};
use crate::test_support::{
    db_supports_public_api_geo_schema, seed_vpn_range, seed_workspace, test_pool, workspace_access,
};

#[tokio::test]
async fn list_events_filters_by_geo_risk_and_network_block_reason() {
    let pool = test_pool();
    if !db_supports_public_api_geo_schema(&pool).await {
        eprintln!("skipping test: local database is missing current audit geo schema");
        return;
    }
    let key = format!("audit-filters-{}", Uuid::new_v4());
    let (principal_id, workspace_id, tenant_id) = seed_workspace(&pool, &key).await;
    seed_vpn_range(&pool, &key).await;

    record_event(
        &pool,
        AuditRecordInput {
            workspace_id,
            actor_user_id: Some(principal_id),
            actor_principal_id: Some(principal_id),
            action: "api.request.denied",
            target_type: "api_key",
            target_id: None,
            ip: Some("8.8.4.42"),
            user_agent: Some("audit-filter-test"),
            metadata: serde_json::json!({
                "request_id": key,
                "network_block_reason": "vpn"
            }),
        },
    )
    .await
    .expect("audit event should be persisted");

    let access = workspace_access(principal_id, workspace_id, tenant_id);
    let response = list_events(
        &pool,
        &access,
        ListAuditEventsInput {
            limit: Some(10),
            before_id: None,
            action: Some("api.request.denied".to_string()),
            actor_user_id: None,
            actor_principal_id: None,
            geo_network_kind: Some("vpn".to_string()),
            min_geo_risk_score: Some(90),
            geo_risk_label: Some("VPN".to_string()),
            network_block_reason: Some("vpn".to_string()),
        },
    )
    .await
    .expect("audit events should be listed");

    assert_eq!(response.workspace_id, workspace_id);
    assert_eq!(response.events.len(), 1);
    let event = &response.events[0];
    assert_eq!(event.geo_country_code.as_deref(), Some("FR"));
    assert_eq!(event.geo_network_kind.as_deref(), Some("vpn"));
    assert_eq!(event.geo_risk_score, Some(95));
    assert!(event.geo_risk_labels.contains(&"vpn".to_string()));
    assert_eq!(event.network_block_reason.as_deref(), Some("vpn"));
}

#[tokio::test]
async fn audit_events_store_geo_metadata_for_quota_billing_and_share_link_actions() {
    let pool = test_pool();
    if !db_supports_public_api_geo_schema(&pool).await {
        eprintln!("skipping test: local database is missing current audit geo schema");
        return;
    }
    let key = format!("audit-domain-geo-{}", Uuid::new_v4());
    let (principal_id, workspace_id, _tenant_id) = seed_workspace(&pool, &key).await;
    seed_vpn_range(&pool, &key).await;

    let storage_object_id = Uuid::new_v4();
    let mut tx = pool.begin().await.expect("transaction should begin");
    record_bandwidth_out_tx(
        &mut tx,
        BandwidthOutUsageInput {
            workspace_id,
            actor_user_id: Some(principal_id),
            actor_principal_id: Some(principal_id),
            storage_object_id,
            source: "audit-domain-geo-test",
            size_bytes: 512,
            ip: Some("8.8.4.42"),
            user_agent: Some("audit-domain-geo-test"),
            idempotency_key: format!("{key}:bandwidth"),
        },
    )
    .await
    .expect("quota audit should be persisted");
    tx.commit().await.expect("transaction should commit");

    for (action, target_type, ip) in [
        ("billing.checkout_completed", "billing", None),
        ("share_link.created", "share_link", Some("8.8.4.42")),
    ] {
        record_event(
            &pool,
            AuditRecordInput {
                workspace_id,
                actor_user_id: Some(principal_id),
                actor_principal_id: Some(principal_id),
                action,
                target_type,
                target_id: Some(workspace_id),
                ip,
                user_agent: Some("audit-domain-geo-test"),
                metadata: serde_json::json!({"source": "audit_domain_geo_test"}),
            },
        )
        .await
        .expect("domain audit event should be persisted");
    }

    let actions = [
        "quota.bandwidth_out_recorded",
        "billing.checkout_completed",
        "share_link.created",
    ];
    for action in actions {
        let row = sqlx::query(
            r#"
            SELECT metadata
            FROM audit_events
            WHERE workspace_id = $1 AND action = $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(workspace_id)
        .bind(action)
        .fetch_one(&pool)
        .await
        .expect("audit event should exist");
        let metadata: sqlx::types::Json<serde_json::Value> = row.get("metadata");
        assert!(
            metadata["geo"].is_object(),
            "{action} should include metadata.geo"
        );
        assert!(metadata["geo"]["geo_source"].is_string());
        assert!(metadata["geo"]["geo_network_kind"].is_string());
        assert!(metadata["geo"]["geo_risk_labels"].is_array());
    }
}
