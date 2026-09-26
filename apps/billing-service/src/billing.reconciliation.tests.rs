use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use super::record_reconciliation_item;
use crate::database::http_test_support::test_router;

#[sqlx::test(migrations = "./migrations")]
async fn records_pending_reconciliation_item(pool: PgPool) {
    let account_id = Uuid::new_v4();
    let id = record_reconciliation_item(
        &pool,
        Some("evt_recon"),
        Some(account_id),
        "orphan_invoice",
        &json!({ "invoice_id": "in_1" }),
    )
    .await
    .expect("insert");

    let status: String =
        sqlx::query_scalar("SELECT status FROM billing_reconciliation_items WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .expect("status");
    assert_eq!(status, "pending");
}

#[sqlx::test(migrations = "./migrations")]
async fn operator_endpoints_require_operator_token(pool: PgPool) {
    let app = test_router(pool);
    let unauthorized = app
        .clone()
        .oneshot(
            Request::get("/operator/billing/overview")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn operator_overview_list_and_resolve(pool: PgPool) {
    let account_id = Uuid::new_v4();
    let item_id = record_reconciliation_item(
        &pool,
        Some("evt_ops"),
        Some(account_id),
        "manual_review",
        &json!({ "note": "test" }),
    )
    .await
    .expect("insert");

    let app = test_router(pool.clone());
    let overview = app
        .clone()
        .oneshot(
            Request::get("/operator/billing/overview")
                .header("authorization", "Bearer operator_fixture")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(overview.status(), StatusCode::OK);
    let overview_json: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(overview.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(overview_json["pending_reconciliations"].as_i64().unwrap() >= 1);

    let list = app
        .clone()
        .oneshot(
            Request::get("/operator/billing/reconciliations")
                .header("authorization", "Bearer operator_fixture")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list.status(), StatusCode::OK);
    let items: Vec<serde_json::Value> = serde_json::from_slice(
        &axum::body::to_bytes(list.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(items.iter().any(|row| row["id"] == item_id.to_string()));

    let resolved = app
        .oneshot(
            Request::post(format!(
                "/operator/billing/reconciliations/{item_id}/resolve"
            ))
            .header("authorization", "Bearer operator_fixture")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({ "resolution_notes": "fixed" }).to_string(),
            ))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resolved.status(), StatusCode::OK);
    let resolved_json: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(resolved.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(resolved_json["status"], "resolved");

    let audits: i64 =
        sqlx::query_scalar("SELECT count(*) FROM billing_audit_events WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(audits, 1);
}
