use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

use super::db::AccessReviewItemRow;
use crate::http::error::AppError;

#[derive(Debug, FromRow)]
pub struct ReviewableCampaignRow {
    pub status: String,
}

pub async fn reviewable_campaign(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    campaign_id: Uuid,
) -> Result<Option<ReviewableCampaignRow>, AppError> {
    Ok(sqlx::query_as::<_, ReviewableCampaignRow>(
        r#"
        SELECT status::text AS status
        FROM access_review_campaigns
        WHERE tenant_id = $1 AND id = $2
        FOR UPDATE
        "#,
    )
    .bind(tenant_id)
    .bind(campaign_id)
    .fetch_optional(&mut **tx)
    .await?)
}

pub async fn get_item_for_update(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    campaign_id: Uuid,
    item_id: Uuid,
) -> Result<Option<AccessReviewItemRow>, AppError> {
    Ok(sqlx::query_as::<_, AccessReviewItemRow>(
        r#"
        SELECT id, item_type::text AS item_type, subject_id, subject_label, workspace_id, role,
          status, evidence, decision::text AS decision, reviewed_by, reviewed_at, created_at
        FROM access_review_items
        WHERE tenant_id = $1 AND campaign_id = $2 AND id = $3
        FOR UPDATE
        "#,
    )
    .bind(tenant_id)
    .bind(campaign_id)
    .bind(item_id)
    .fetch_optional(&mut **tx)
    .await?)
}

pub async fn update_item_decision(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    campaign_id: Uuid,
    item_id: Uuid,
    reviewer_id: Uuid,
    decision: &str,
    note: Option<&str>,
) -> Result<Option<AccessReviewItemRow>, AppError> {
    Ok(sqlx::query_as::<_, AccessReviewItemRow>(
        r#"
        UPDATE access_review_items
        SET decision = $5::access_review_item_decision,
          reviewed_by = $4,
          reviewed_at = NOW(),
          evidence = CASE
            WHEN $6::text IS NULL THEN evidence
            ELSE jsonb_set(evidence, '{review_note}', to_jsonb($6::text), true)
          END
        WHERE tenant_id = $1 AND campaign_id = $2 AND id = $3
        RETURNING id, item_type::text AS item_type, subject_id, subject_label, workspace_id, role,
          status, evidence, decision::text AS decision, reviewed_by, reviewed_at, created_at
        "#,
    )
    .bind(tenant_id)
    .bind(campaign_id)
    .bind(item_id)
    .bind(reviewer_id)
    .bind(decision)
    .bind(note)
    .fetch_optional(&mut **tx)
    .await?)
}

pub async fn pending_item_count(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    campaign_id: Uuid,
) -> Result<i64, AppError> {
    Ok(sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)::bigint
        FROM access_review_items
        WHERE tenant_id = $1 AND campaign_id = $2 AND decision = 'pending'
        "#,
    )
    .bind(tenant_id)
    .bind(campaign_id)
    .fetch_one(&mut **tx)
    .await?)
}

pub async fn close_campaign(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    campaign_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE access_review_campaigns
        SET status = 'closed', closed_at = NOW()
        WHERE tenant_id = $1 AND id = $2 AND status = 'active'
        "#,
    )
    .bind(tenant_id)
    .bind(campaign_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use sqlx::{PgPool, Row};

    #[tokio::test]
    #[ignore = "requires a reachable PostgreSQL test database"]
    async fn close_campaign_marks_active_campaign_closed_after_last_decision() {
        let _guard = crate::test_support::test_database_lock().lock().await;
        let pool = crate::test_support::isolated_test_pool(5);
        crate::test_support::ensure_test_database(&pool).await;
        let fixture = seed_decision_fixture(&pool).await;

        let mut tx = pool.begin().await.expect("transaction should start");
        let item = get_item_for_update(
            &mut tx,
            fixture.tenant_id,
            fixture.campaign_id,
            fixture.item_id,
        )
        .await
        .expect("item lookup should succeed")
        .expect("item should exist");

        assert_eq!(item.decision, "pending");

        let updated = update_item_decision(
            &mut tx,
            fixture.tenant_id,
            fixture.campaign_id,
            fixture.item_id,
            fixture.reviewer_id,
            "approved",
            None,
        )
        .await
        .expect("decision update should succeed")
        .expect("updated item should be returned");

        assert_eq!(updated.decision, "approved");
        assert_eq!(
            pending_item_count(&mut tx, fixture.tenant_id, fixture.campaign_id)
                .await
                .expect("pending count should be readable"),
            0
        );

        close_campaign(&mut tx, fixture.tenant_id, fixture.campaign_id)
            .await
            .expect("campaign close should succeed");
        tx.commit().await.expect("transaction should commit");

        let status = sqlx::query_scalar::<_, String>(
            "SELECT status::text FROM access_review_campaigns WHERE id = $1",
        )
        .bind(fixture.campaign_id)
        .fetch_one(&pool)
        .await
        .expect("campaign status should be readable");

        assert_eq!(status, "closed");
        cleanup_fixture(&pool, fixture.tenant_id).await;
    }

    struct DecisionFixture {
        tenant_id: Uuid,
        reviewer_id: Uuid,
        campaign_id: Uuid,
        item_id: Uuid,
    }

    async fn seed_decision_fixture(pool: &PgPool) -> DecisionFixture {
        let tenant_id = Uuid::new_v4();
        let reviewer_id = Uuid::new_v4();
        let campaign_id = Uuid::new_v4();
        let item_id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
            VALUES ($1, 'team', 'Access Review Decision Test Tenant', $2, 'active', 'standard', $3, $3)
            "#,
        )
        .bind(tenant_id)
        .bind(format!("access-review-decision-test-{tenant_id}"))
        .bind(now)
        .execute(pool)
        .await
        .expect("tenant insert should succeed");

        sqlx::query(
            r#"
            INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
            VALUES ($1, $2, 'human', 'active', 'Reviewer', $3, $3)
            "#,
        )
        .bind(reviewer_id)
        .bind(tenant_id)
        .bind(now)
        .execute(pool)
        .await
        .expect("principal insert should succeed");

        sqlx::query(
            r#"
            INSERT INTO access_review_campaigns (id, tenant_id, name, due_at, created_by, created_at)
            VALUES ($1, $2, 'Decision test', $3, $4, $5)
            "#,
        )
        .bind(campaign_id)
        .bind(tenant_id)
        .bind(now + Duration::days(7))
        .bind(reviewer_id)
        .bind(now)
        .execute(pool)
        .await
        .expect("campaign insert should succeed");

        sqlx::query(
            r#"
            INSERT INTO access_review_items (
              id, campaign_id, tenant_id, item_type, subject_id, subject_label, status, evidence
            )
            VALUES ($1, $2, $3, 'member', $4, 'Reviewer', 'active', '{}'::jsonb)
            "#,
        )
        .bind(item_id)
        .bind(campaign_id)
        .bind(tenant_id)
        .bind(reviewer_id.to_string())
        .execute(pool)
        .await
        .expect("item insert should succeed");

        let count = sqlx::query(
            "SELECT COUNT(*)::bigint AS count FROM access_review_items WHERE campaign_id = $1",
        )
        .bind(campaign_id)
        .fetch_one(pool)
        .await
        .expect("seed count should be readable")
        .get::<i64, _>("count");
        assert_eq!(count, 1);

        DecisionFixture {
            tenant_id,
            reviewer_id,
            campaign_id,
            item_id,
        }
    }

    async fn cleanup_fixture(pool: &PgPool, tenant_id: Uuid) {
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .execute(pool)
            .await
            .expect("tenant cleanup should succeed");
    }
}
