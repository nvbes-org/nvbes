use uuid::Uuid;

use super::{
    DUE_SCHEDULE_LIMIT, REMINDER_LIMIT, claim_access_review_reminder_candidates,
    materialize_due_access_review_schedules, request_context,
};

#[tokio::test]
async fn disabled_enterprise_integration_is_a_successful_no_op() {
    assert!(
        materialize_due_access_review_schedules(None)
            .await
            .expect("schedule no-op")
            .is_none()
    );
    assert!(
        claim_access_review_reminder_candidates(None)
            .await
            .expect("reminder no-op")
            .is_none()
    );
}

#[test]
fn system_request_context_uses_unique_requests_and_nil_tenant_actor() {
    let first = request_context();
    let second = request_context();
    let nil = Uuid::nil().to_string();

    assert_ne!(first.request_id, second.request_id);
    assert_ne!(first.correlation_id, second.correlation_id);
    assert_eq!(first.actor_principal_id, nil);
    let tenant = first.tenant.expect("tenant context");
    assert_eq!(tenant.tenant_id, nil);
    assert!(tenant.workspace_id.is_empty());
    assert!(tenant.region_id.is_empty());
    assert!(tenant.data_residency.is_empty());
    assert_eq!(DUE_SCHEDULE_LIMIT, 25);
    assert_eq!(REMINDER_LIMIT, 100);
}
