use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    grpc::pb::nvbes::enterprise::v1 as enterprise,
    test_support::{
        cleanup_tenant, has_enterprise_contract_schema, membership_status, seed_enterprise_fixture,
        test_pool,
    },
};

#[tokio::test]
async fn start_access_review_snapshots_member_scope() {
    let pool = test_pool();
    if !has_enterprise_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Enterprise contract schema");
        return;
    }

    let fixture = seed_enterprise_fixture(&pool, "access-review-start").await;
    let due_at = (Utc::now() + Duration::days(7)).to_rfc3339();
    let review = super::start_access_review(
        &pool,
        fixture.tenant_id,
        fixture.actor_id,
        enterprise::StartAccessReviewRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            scope: "members".to_string(),
            due_at: due_at.clone(),
            name: "Member review".to_string(),
            description: String::new(),
            include_members: false,
            include_roles: false,
            include_service_accounts: false,
            include_oauth_clients: false,
        },
    )
    .await
    .expect("access review should start");

    assert_eq!(review.tenant_id, fixture.tenant_id.to_string());
    assert_eq!(review.scope, "members");
    assert_eq!(review.status, "active");
    assert_eq!(review.due_at, due_at);
    assert!(Uuid::parse_str(&review.review_id).is_ok());

    let item_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)::bigint FROM access_review_items WHERE campaign_id = $1",
    )
    .bind(Uuid::parse_str(&review.review_id).expect("review id should be UUID"))
    .fetch_one(&pool)
    .await
    .expect("review items should be countable");
    assert_eq!(item_count, 2);

    let listed = super::reads::list_access_reviews(&pool, fixture.tenant_id)
        .await
        .expect("access reviews should be listed");
    assert!(
        listed
            .campaigns
            .iter()
            .any(|candidate| candidate.review_id == review.review_id)
    );

    let detail = super::reads::get_access_review(&pool, fixture.tenant_id, &review.review_id)
        .await
        .expect("access review detail should be readable");
    let campaign = detail.campaign.expect("campaign summary should be present");
    assert_eq!(campaign.review_id, review.review_id);
    assert_eq!(campaign.pending_items, 2);
    assert_eq!(detail.items.len(), 2);

    cleanup_tenant(&pool, fixture.tenant_id).await;
}

#[tokio::test]
async fn revoked_access_review_decision_removes_member_access() {
    let pool = test_pool();
    if !has_enterprise_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Enterprise contract schema");
        return;
    }

    let fixture = seed_enterprise_fixture(&pool, "access-review-revoke").await;
    let review = super::start_access_review(
        &pool,
        fixture.tenant_id,
        fixture.actor_id,
        enterprise::StartAccessReviewRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            scope: "members".to_string(),
            due_at: (Utc::now() + Duration::days(7)).to_rfc3339(),
            name: "Revocation review".to_string(),
            description: String::new(),
            include_members: false,
            include_roles: false,
            include_service_accounts: false,
            include_oauth_clients: false,
        },
    )
    .await
    .expect("access review should start");

    let decision = super::record_access_review_decision(
        &pool,
        fixture.tenant_id,
        fixture.actor_id,
        enterprise::RecordAccessReviewDecisionRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            review_id: review.review_id,
            subject_principal_id: fixture.principal_id.to_string(),
            decision: "revoked".to_string(),
            reason: "contract test revocation".to_string(),
            item_id: String::new(),
            target_role: String::new(),
        },
    )
    .await
    .expect("access review decision should be recorded");

    assert_eq!(
        decision.subject_principal_id,
        fixture.principal_id.to_string()
    );
    assert_eq!(decision.decision, "revoked");
    assert_eq!(
        membership_status(&pool, fixture.tenant_id, fixture.principal_id).await,
        "removed"
    );

    cleanup_tenant(&pool, fixture.tenant_id).await;
}

#[tokio::test]
async fn changed_access_review_decision_updates_workspace_role() {
    let pool = test_pool();
    if !has_enterprise_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Enterprise contract schema");
        return;
    }

    let fixture = seed_enterprise_fixture(&pool, "access-review-change").await;
    let workspace_id = seed_workspace_membership(&pool, &fixture, "member").await;
    let review = super::start_access_review(
        &pool,
        fixture.tenant_id,
        fixture.actor_id,
        enterprise::StartAccessReviewRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            scope: "roles".to_string(),
            due_at: (Utc::now() + Duration::days(7)).to_rfc3339(),
            name: "Role change review".to_string(),
            description: String::new(),
            include_members: false,
            include_roles: false,
            include_service_accounts: false,
            include_oauth_clients: false,
        },
    )
    .await
    .expect("access review should start");
    let item_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM access_review_items
        WHERE campaign_id = $1 AND workspace_id = $2 AND subject_id LIKE $3
        "#,
    )
    .bind(Uuid::parse_str(&review.review_id).expect("review id should be UUID"))
    .bind(workspace_id)
    .bind(format!("{}:%", fixture.principal_id))
    .fetch_one(&pool)
    .await
    .expect("role item should exist");

    let decision = super::record_access_review_decision(
        &pool,
        fixture.tenant_id,
        fixture.actor_id,
        enterprise::RecordAccessReviewDecisionRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            review_id: review.review_id,
            subject_principal_id: String::new(),
            decision: "changed".to_string(),
            reason: "contract test role change".to_string(),
            item_id: item_id.to_string(),
            target_role: "viewer".to_string(),
        },
    )
    .await
    .expect("access review decision should be recorded");

    assert_eq!(decision.decision, "changed");
    assert_eq!(
        workspace_role(&pool, workspace_id, fixture.principal_id).await,
        "viewer"
    );

    cleanup_tenant(&pool, fixture.tenant_id).await;
}

#[tokio::test]
async fn close_access_review_closes_active_campaign_with_reason() {
    let pool = test_pool();
    if !has_enterprise_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Enterprise contract schema");
        return;
    }

    let fixture = seed_enterprise_fixture(&pool, "access-review-close").await;
    let review = super::start_access_review(
        &pool,
        fixture.tenant_id,
        fixture.actor_id,
        enterprise::StartAccessReviewRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            scope: "members".to_string(),
            due_at: (Utc::now() + Duration::days(7)).to_rfc3339(),
            name: "Close review".to_string(),
            description: String::new(),
            include_members: false,
            include_roles: false,
            include_service_accounts: false,
            include_oauth_clients: false,
        },
    )
    .await
    .expect("access review should start");

    let closed = super::close_access_review(
        &pool,
        fixture.tenant_id,
        enterprise::CloseAccessReviewRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            review_id: review.review_id,
            reason: "manual close contract test".to_string(),
        },
    )
    .await
    .expect("access review should close");

    assert_eq!(closed.status, "closed");
    cleanup_tenant(&pool, fixture.tenant_id).await;
}

#[tokio::test]
async fn access_review_schedule_create_list_disable_and_run_round_trip() {
    let pool = test_pool();
    if !has_enterprise_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Enterprise contract schema");
        return;
    }

    let fixture = seed_enterprise_fixture(&pool, "access-review-schedule").await;
    let schedule = super::schedules::create_access_review_schedule(
        &pool,
        fixture.tenant_id,
        fixture.actor_id,
        enterprise::CreateAccessReviewScheduleRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            name: "Quarterly member review".to_string(),
            description: "Review tenant members".to_string(),
            scope: "members".to_string(),
            recurrence_days: 90,
            due_after_days: 14,
            include_members: true,
            include_roles: false,
            include_service_accounts: false,
            include_oauth_clients: false,
        },
    )
    .await
    .expect("access review schedule should be created");

    assert_eq!(schedule.tenant_id, fixture.tenant_id.to_string());
    assert_eq!(schedule.scope, "members");
    assert_eq!(schedule.recurrence_days, 90);
    assert_eq!(schedule.due_after_days, 14);
    assert!(schedule.disabled_at.is_empty());

    let listed = super::schedules::list_access_review_schedules(&pool, fixture.tenant_id)
        .await
        .expect("access review schedules should be listed");
    assert!(
        listed
            .schedules
            .iter()
            .any(|candidate| candidate.schedule_id == schedule.schedule_id)
    );

    let disabled = super::schedules::set_access_review_schedule_enabled(
        &pool,
        fixture.tenant_id,
        Uuid::parse_str(&schedule.schedule_id).expect("schedule id should be UUID"),
        false,
    )
    .await
    .expect("schedule should be disabled");
    assert!(!disabled.disabled_at.is_empty());

    let enabled = super::schedules::set_access_review_schedule_enabled(
        &pool,
        fixture.tenant_id,
        Uuid::parse_str(&schedule.schedule_id).expect("schedule id should be UUID"),
        true,
    )
    .await
    .expect("schedule should be enabled");
    assert!(enabled.disabled_at.is_empty());

    let review = super::schedules::run_access_review_schedule(
        &pool,
        fixture.tenant_id,
        Uuid::parse_str(&schedule.schedule_id).expect("schedule id should be UUID"),
        fixture.actor_id,
    )
    .await
    .expect("schedule should create a campaign");
    assert_eq!(review.tenant_id, fixture.tenant_id.to_string());
    assert_eq!(review.scope, "members");

    let stored_last_review = sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT last_campaign_id FROM access_review_schedules WHERE id = $1",
    )
    .bind(Uuid::parse_str(&schedule.schedule_id).expect("schedule id should be UUID"))
    .fetch_one(&pool)
    .await
    .expect("last campaign should be readable")
    .expect("schedule should store last campaign");
    assert_eq!(stored_last_review.to_string(), review.review_id);

    let due_schedule = super::schedules::create_access_review_schedule(
        &pool,
        fixture.tenant_id,
        fixture.actor_id,
        enterprise::CreateAccessReviewScheduleRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            name: "Due member review".to_string(),
            description: String::new(),
            scope: "members".to_string(),
            recurrence_days: 30,
            due_after_days: 7,
            include_members: true,
            include_roles: false,
            include_service_accounts: false,
            include_oauth_clients: false,
        },
    )
    .await
    .expect("due access review schedule should be created");
    let run = super::schedules::materialize_due_schedules(
        &pool,
        enterprise::MaterializeDueAccessReviewSchedulesRequest {
            context: None,
            limit: 10,
        },
    )
    .await
    .expect("due schedules should be materialized by Enterprise");
    assert_eq!(run.campaigns_created, 1);
    assert_eq!(run.empty_schedules, 0);

    let materialized_last_review = sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT last_campaign_id FROM access_review_schedules WHERE id = $1",
    )
    .bind(Uuid::parse_str(&due_schedule.schedule_id).expect("schedule id should be UUID"))
    .fetch_one(&pool)
    .await
    .expect("materialized schedule should be readable")
    .expect("materialized schedule should store last campaign");
    let materialized_audit_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM audit_events
        WHERE tenant_id = $1
          AND action = 'enterprise.access_review_campaign.created'
          AND target_id = $2
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(materialized_last_review)
    .fetch_one(&pool)
    .await
    .expect("materialized schedule audit should be readable");
    assert_eq!(materialized_audit_count, 1);

    cleanup_tenant(&pool, fixture.tenant_id).await;
}

async fn seed_workspace_membership(
    pool: &sqlx::PgPool,
    fixture: &crate::test_support::EnterpriseFixture,
    subject_role: &str,
) -> Uuid {
    let workspace_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO workspaces (
          id, tenant_id, name, workspace_type, owner_user_id, plan_code, created_at, updated_at
        )
        VALUES ($1, $2, 'Enterprise contract workspace', 'team', $3, 'team', $4, $4)
        "#,
    )
    .bind(workspace_id)
    .bind(fixture.tenant_id)
    .bind(fixture.actor_id)
    .bind(fixture.now)
    .execute(pool)
    .await
    .expect("workspace should be seeded");

    sqlx::query(
        r#"
        INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, source, created_at, updated_at)
        VALUES
          ($1, $2, 'owner', 'active', 'manual', $4, $4),
          ($1, $3, $5::workspace_member_role, 'active', 'manual', $4, $4)
        "#,
    )
    .bind(workspace_id)
    .bind(fixture.actor_id)
    .bind(fixture.principal_id)
    .bind(fixture.now)
    .bind(subject_role)
    .execute(pool)
    .await
    .expect("workspace memberships should be seeded");
    workspace_id
}

async fn workspace_role(pool: &sqlx::PgPool, workspace_id: Uuid, principal_id: Uuid) -> String {
    sqlx::query_scalar(
        "SELECT role::text FROM workspace_memberships WHERE workspace_id = $1 AND principal_id = $2",
    )
    .bind(workspace_id)
    .bind(principal_id)
    .fetch_one(pool)
    .await
    .expect("workspace role should be readable")
}
