use sqlx::Transaction;
use uuid::Uuid;

use crate::error::AppError;

pub(crate) struct RoutingRuleConflictInput<'a> {
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
          AND (country IS NULL OR $1::text IS NULL OR country::text = $1)
          AND (currency IS NULL OR $2::text IS NULL OR currency::text = $2)
          AND (payment_method IS NULL OR $3::text IS NULL OR payment_method = $3)
          AND (customer_type IS NULL OR $4::text IS NULL OR customer_type = $4)
          AND COALESCE(min_amount_minor, 0) <= COALESCE($6, 9223372036854775807)
          AND COALESCE(max_amount_minor, 9223372036854775807) >= COALESCE($5, 0)
        ORDER BY priority ASC, updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(input.country)
    .bind(input.currency)
    .bind(input.payment_method)
    .bind(input.customer_type)
    .bind(input.min_amount_minor)
    .bind(input.max_amount_minor)
    .fetch_optional(tx.as_mut())
    .await?;

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
