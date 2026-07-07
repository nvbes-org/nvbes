use uuid::Uuid;

use crate::{
    grpc::{access_reviews::reminders, pb::nvbes::enterprise::v1 as enterprise},
    test_support::{
        cleanup_tenant, has_enterprise_contract_schema, seed_enterprise_fixture, test_pool,
    },
};

#[tokio::test]
async fn reminder_candidates_are_claimed_by_enterprise_once() {
    let pool = test_pool();
    if !has_enterprise_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Enterprise contract schema");
        return;
    }

    let fixture = seed_enterprise_fixture(&pool, "access-review-reminders").await;
    seed_owner_user(&pool, &fixture).await;
    let campaign_id = seed_due_campaign(&pool, &fixture).await;
    seed_pending_item(&pool, &fixture, campaign_id).await;

    let first_claim = reminders::claim_reminder_candidates(
        &pool,
        enterprise::ClaimAccessReviewReminderCandidatesRequest {
            context: None,
            limit: 10,
        },
    )
    .await
    .expect("reminder candidates should be claimed");
    assert_eq!(first_claim.candidates.len(), 1);
    let candidate = &first_claim.candidates[0];
    assert_eq!(candidate.tenant_id, fixture.tenant_id.to_string());
    assert_eq!(candidate.campaign_id, campaign_id.to_string());
    assert_eq!(candidate.recipient_email, "owner@example.test");
    assert_eq!(candidate.reminder_kind, "due_soon");

    let second_claim = reminders::claim_reminder_candidates(
        &pool,
        enterprise::ClaimAccessReviewReminderCandidatesRequest {
            context: None,
            limit: 10,
        },
    )
    .await
    .expect("claimed reminders should not be returned again");
    assert!(second_claim.candidates.is_empty());

    let reminder_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM access_review_reminders
        WHERE tenant_id = $1
          AND campaign_id = $2
          AND recipient_principal_id = $3
          AND reminder_kind = 'due_soon'
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(campaign_id)
    .bind(fixture.actor_id)
    .fetch_one(&pool)
    .await
    .expect("reminder claim should be readable");
    assert_eq!(reminder_count, 1);

    cleanup_tenant(&pool, fixture.tenant_id).await;
}

async fn seed_owner_user(pool: &sqlx::PgPool, fixture: &crate::test_support::EnterpriseFixture) {
    sqlx::query(
        r#"
        UPDATE tenant_memberships
        SET role = 'owner'
        WHERE tenant_id = $1 AND principal_id = $2
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(fixture.actor_id)
    .execute(pool)
    .await
    .expect("actor should be promoted to owner");

    sqlx::query(
        r#"
        INSERT INTO users (
          principal_id, email, name, firstname, lastname, username, status, created_at, updated_at
        )
        VALUES ($1, 'owner@example.test', 'Owner User', 'Owner', 'User', 'owner', 'active', $2, $2)
        "#,
    )
    .bind(fixture.actor_id)
    .bind(fixture.now)
    .execute(pool)
    .await
    .expect("owner user should be seeded");
}

async fn seed_due_campaign(
    pool: &sqlx::PgPool,
    fixture: &crate::test_support::EnterpriseFixture,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO access_review_campaigns (tenant_id, name, due_at, created_by)
        VALUES ($1, 'Quarterly review', $2 + INTERVAL '1 day', $3)
        RETURNING id
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(fixture.now)
    .bind(fixture.actor_id)
    .fetch_one(pool)
    .await
    .expect("due campaign should be seeded")
}

async fn seed_pending_item(
    pool: &sqlx::PgPool,
    fixture: &crate::test_support::EnterpriseFixture,
    campaign_id: Uuid,
) {
    sqlx::query(
        r#"
        INSERT INTO access_review_items (
          campaign_id, tenant_id, item_type, subject_id, subject_label, role, status
        )
        VALUES ($1, $2, 'member', $3, 'Subject principal', 'member', 'active')
        "#,
    )
    .bind(campaign_id)
    .bind(fixture.tenant_id)
    .bind(fixture.principal_id.to_string())
    .execute(pool)
    .await
    .expect("pending item should be seeded");
}
