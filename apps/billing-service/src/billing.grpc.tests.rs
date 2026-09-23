use nvbes_billing::proto::nvbes::billing::v1::{
    CreateBillingPortalSessionRequest, CreateCheckoutSessionRequest,
    GetBillingOperationsSnapshotRequest, GetWorkspaceBillingOverviewRequest,
    GetWorkspaceEntitlementsRequest, ReconcileWorkspaceRequest, SubmitBillingEventRequest,
    billing_delivery_service_server::BillingDeliveryService,
    billing_operations_service_server::BillingOperationsService,
};
use serde_json::json;
use sqlx::PgPool;
use tonic::{Code, Request};
use uuid::Uuid;

use crate::{
    app::BillingState,
    auth::TokenVerifier,
    database::test_support::test_config,
    grpc::{BillingDeliveryGrpcService, BillingOperationsGrpcService},
    metrics::install,
};

fn state(pool: PgPool) -> BillingState {
    let config = test_config();
    let tokens = TokenVerifier::new(&config).expect("verifier");
    BillingState {
        db: pool,
        config,
        metrics: install(),
        tokens,
        email_client: None,
    }
}

fn authed_request<T>(message: T) -> Request<T> {
    let mut request = Request::new(message);
    request
        .metadata_mut()
        .insert("authorization", "Bearer operator_fixture".parse().unwrap());
    request
}

#[sqlx::test(migrations = "./migrations")]
async fn delivery_requires_operator_token(pool: PgPool) {
    let service = BillingDeliveryGrpcService::new(state(pool));
    let err = service
        .get_workspace_entitlements(Request::new(GetWorkspaceEntitlementsRequest {
            context: None,
            workspace_id: Uuid::new_v4().to_string(),
        }))
        .await
        .expect_err("unauthenticated");
    assert_eq!(err.code(), Code::Unauthenticated);
}

#[sqlx::test(migrations = "./migrations")]
async fn delivery_entitlements_and_overview(pool: PgPool) {
    let workspace = Uuid::new_v4();
    let customer_id = format!("cus_{}", Uuid::new_v4().simple());
    let sub_id = format!("sub_{}", Uuid::new_v4().simple());

    sqlx::query(
        "INSERT INTO billing_customers (account_id, account_type, stripe_customer_id, email)
         VALUES ($1, 'team', $2, 'grpc@t.test')",
    )
    .bind(workspace)
    .bind(&customer_id)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO billing_subscriptions (
            account_id, account_type, stripe_subscription_id, stripe_customer_id,
            plan_code, status, monthly_price_cents, cancel_at_period_end
         ) VALUES ($1, 'team', $2, $3, 'standard_monthly', 'active', 1000, false)",
    )
    .bind(workspace)
    .bind(&sub_id)
    .bind(&customer_id)
    .execute(&pool)
    .await
    .unwrap();

    let service = BillingDeliveryGrpcService::new(state(pool));

    let entitlements = service
        .get_workspace_entitlements(authed_request(GetWorkspaceEntitlementsRequest {
            context: None,
            workspace_id: workspace.to_string(),
        }))
        .await
        .expect("entitlements")
        .into_inner();
    assert_eq!(entitlements.plan_code, "standard_monthly");
    assert_eq!(entitlements.api_rate_limit_per_minute, 0);

    let overview = service
        .get_workspace_billing_overview(authed_request(GetWorkspaceBillingOverviewRequest {
            context: None,
            workspace_id: workspace.to_string(),
        }))
        .await
        .expect("overview")
        .into_inner();
    assert_eq!(overview.monthly_price_cents, 1000);
    assert_eq!(overview.currency, "eur");
}

#[sqlx::test(migrations = "./migrations")]
async fn delivery_checkout_portal_and_outbox(pool: PgPool) {
    let workspace = Uuid::new_v4();
    let service = BillingDeliveryGrpcService::new(state(pool.clone()));

    let checkout = service
        .create_checkout_session(authed_request(CreateCheckoutSessionRequest {
            context: None,
            workspace_id: workspace.to_string(),
            plan_code: "standard_monthly".into(),
            success_url: "https://nvbes.test/success".into(),
            cancel_url: "https://nvbes.test/cancel".into(),
            idempotency_key: "grpc-checkout".into(),
            customer_email: None,
        }))
        .await
        .expect("checkout")
        .into_inner();
    assert_eq!(checkout.status, "open");
    assert!(checkout.checkout_url.contains("mock-checkout"));

    let portal = service
        .create_billing_portal_session(authed_request(CreateBillingPortalSessionRequest {
            context: None,
            workspace_id: workspace.to_string(),
            return_url: "https://nvbes.test/billing".into(),
        }))
        .await
        .expect("portal")
        .into_inner();
    assert!(portal.portal_url.contains("mock-portal"));

    let receipt = service
        .submit_billing_event(authed_request(SubmitBillingEventRequest {
            context: None,
            event_type: "billing.subscription.changed.v1".into(),
            aggregate_id: workspace.to_string(),
            payload_json: json!({ "status": "active" }).to_string(),
            idempotency_key: "grpc-outbox".into(),
        }))
        .await
        .expect("outbox")
        .into_inner();
    assert!(!receipt.duplicate);

    let pending: i64 =
        sqlx::query_scalar("SELECT count(*) FROM billing_outbox WHERE aggregate_id = $1")
            .bind(workspace)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(pending, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn delivery_validates_arguments(pool: PgPool) {
    let service = BillingDeliveryGrpcService::new(state(pool));

    let invalid_uuid = service
        .get_workspace_entitlements(authed_request(GetWorkspaceEntitlementsRequest {
            context: None,
            workspace_id: "not-a-uuid".into(),
        }))
        .await
        .expect_err("bad uuid");
    assert_eq!(invalid_uuid.code(), Code::InvalidArgument);

    let missing_fields = service
        .create_checkout_session(authed_request(CreateCheckoutSessionRequest {
            context: None,
            workspace_id: Uuid::new_v4().to_string(),
            plan_code: "".into(),
            success_url: "".into(),
            cancel_url: "".into(),
            idempotency_key: "k".into(),
            customer_email: None,
        }))
        .await
        .expect_err("missing fields");
    assert_eq!(missing_fields.code(), Code::InvalidArgument);

    let bad_json = service
        .submit_billing_event(authed_request(SubmitBillingEventRequest {
            context: None,
            event_type: "billing.test".into(),
            aggregate_id: Uuid::new_v4().to_string(),
            payload_json: "not-json".into(),
            idempotency_key: "bad-json".into(),
        }))
        .await
        .expect_err("bad json");
    assert_eq!(bad_json.code(), Code::InvalidArgument);
}

#[sqlx::test(migrations = "./migrations")]
async fn operations_snapshot_and_reconcile(pool: PgPool) {
    let workspace = Uuid::new_v4();
    let customer_id = format!("cus_{}", Uuid::new_v4().simple());
    let sub_id = format!("sub_{}", Uuid::new_v4().simple());

    sqlx::query(
        "INSERT INTO billing_customers (account_id, account_type, stripe_customer_id, email)
         VALUES ($1, 'team', $2, 'ops@t.test')",
    )
    .bind(workspace)
    .bind(&customer_id)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO billing_subscriptions (
            account_id, account_type, stripe_subscription_id, stripe_customer_id,
            plan_code, status, monthly_price_cents
         ) VALUES ($1, 'team', $2, $3, 'standard_monthly', 'active', 1000)",
    )
    .bind(workspace)
    .bind(&sub_id)
    .bind(&customer_id)
    .execute(&pool)
    .await
    .unwrap();

    let service = BillingOperationsGrpcService::new(state(pool));

    let snapshot = service
        .get_billing_operations_snapshot(authed_request(GetBillingOperationsSnapshotRequest {
            context: None,
        }))
        .await
        .expect("snapshot")
        .into_inner();
    assert_eq!(snapshot.active_subscriptions_count, 1);
    assert!(snapshot.monthly_recurring_revenue_cents >= 1000);

    let reconcile = service
        .reconcile_workspace(authed_request(ReconcileWorkspaceRequest {
            context: None,
            workspace_id: workspace.to_string(),
        }))
        .await
        .expect("reconcile")
        .into_inner();
    assert_eq!(reconcile.current_status, "active");
    assert!(reconcile.status_synced);
}
