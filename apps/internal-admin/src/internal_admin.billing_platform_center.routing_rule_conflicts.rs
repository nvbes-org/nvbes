use sqlx::{Row, Transaction};
use uuid::Uuid;

use crate::error::AppError;

pub(crate) struct RoutingRuleConflictInput<'a> {
    pub(crate) exclude_rule_id: Option<Uuid>,
    pub(crate) country: Option<&'a str>,
    pub(crate) currency: Option<&'a str>,
    pub(crate) payment_method: Option<&'a str>,
    pub(crate) customer_type: Option<&'a str>,
    pub(crate) min_amount_minor: Option<i64>,
    pub(crate) max_amount_minor: Option<i64>,
}

pub(crate) async fn reject_active_routing_rule_overlap(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    input: RoutingRuleConflictInput<'_>,
) -> Result<(), AppError> {
    let conflicting_rule_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM billing_provider_routing_rules
        WHERE status = 'active'
          AND ($1::uuid IS NULL OR id <> $1)
          AND (country IS NULL OR $2::text IS NULL OR country::text = $2)
          AND (currency IS NULL OR $3::text IS NULL OR currency::text = $3)
          AND (payment_method IS NULL OR $4::text IS NULL OR payment_method = $4)
          AND (customer_type IS NULL OR $5::text IS NULL OR customer_type = $5)
          AND COALESCE(min_amount_minor, 0) <= COALESCE($7, 9223372036854775807)
          AND COALESCE(max_amount_minor, 9223372036854775807) >= COALESCE($6, 0)
        ORDER BY priority ASC, updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(input.exclude_rule_id)
    .bind(input.country)
    .bind(input.currency)
    .bind(input.payment_method)
    .bind(input.customer_type)
    .bind(input.min_amount_minor)
    .bind(input.max_amount_minor)
    .fetch_optional(tx.as_mut())
    .await?;

    reject_conflict(conflicting_rule_id)
}

pub(crate) async fn reject_active_routing_rule_overlap_for_rule(
    tx: &mut Transaction<'_, sqlx::Postgres>,
    rule_id: Uuid,
) -> Result<(), AppError> {
    let row = sqlx::query(
        r#"
        SELECT country::text AS country, currency::text AS currency, payment_method,
          customer_type, min_amount_minor, max_amount_minor
        FROM billing_provider_routing_rules
        WHERE id = $1
        "#,
    )
    .bind(rule_id)
    .fetch_optional(tx.as_mut())
    .await?;

    let Some(row) = row else {
        return Ok(());
    };

    let conflicting_rule_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM billing_provider_routing_rules
        WHERE status = 'active'
          AND id <> $1
          AND (country IS NULL OR $2::text IS NULL OR country::text = $2)
          AND (currency IS NULL OR $3::text IS NULL OR currency::text = $3)
          AND (payment_method IS NULL OR $4::text IS NULL OR payment_method = $4)
          AND (customer_type IS NULL OR $5::text IS NULL OR customer_type = $5)
          AND COALESCE(min_amount_minor, 0) <= COALESCE($7, 9223372036854775807)
          AND COALESCE(max_amount_minor, 9223372036854775807) >= COALESCE($6, 0)
        ORDER BY priority ASC, updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(rule_id)
    .bind(row.get::<Option<String>, _>("country"))
    .bind(row.get::<Option<String>, _>("currency"))
    .bind(row.get::<Option<String>, _>("payment_method"))
    .bind(row.get::<Option<String>, _>("customer_type"))
    .bind(row.get::<Option<i64>, _>("min_amount_minor"))
    .bind(row.get::<Option<i64>, _>("max_amount_minor"))
    .fetch_optional(tx.as_mut())
    .await?;

    reject_conflict(conflicting_rule_id)
}

fn reject_conflict(conflicting_rule_id: Option<Uuid>) -> Result<(), AppError> {
    if let Some(rule_id) = conflicting_rule_id {
        return Err(AppError::conflict(
            "routing_rule_overlap",
            &format!(
                "Routing rule overlaps active rule {rule_id}. Disable the existing rule first."
            ),
        ));
    }
    Ok(())
}
