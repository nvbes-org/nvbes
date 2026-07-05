use sqlx::{PgConnection, Row};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::pb::nvbes::billing::v1::CreateBillingRoutingRuleInput;

pub fn validate_routing_rule(input: &CreateBillingRoutingRuleInput) -> Result<(), Status> {
    if nvbes_billing::provider_code(&input.provider).is_none() {
        return Err(Status::invalid_argument(
            "invalid_routing_provider: Routing provider is not supported.",
        ));
    }
    validate_code_len(&input.country, 2, "invalid_routing_country")?;
    validate_code_len(&input.currency, 3, "invalid_routing_currency")?;
    let min_amount = input.has_min_amount_minor.then_some(input.min_amount_minor);
    let max_amount = input.has_max_amount_minor.then_some(input.max_amount_minor);
    if min_amount.is_some_and(|amount| amount < 0) || max_amount.is_some_and(|amount| amount < 0) {
        return Err(Status::invalid_argument(
            "invalid_routing_amount: Routing amount bounds must be non-negative.",
        ));
    }
    if let (Some(min_amount), Some(max_amount)) = (min_amount, max_amount)
        && min_amount > max_amount
    {
        return Err(Status::invalid_argument(
            "invalid_routing_amount_range: Routing minimum amount cannot exceed maximum amount.",
        ));
    }
    Ok(())
}

pub async fn reject_active_routing_rule_overlap(
    executor: &mut PgConnection,
    exclude_rule_id: Option<Uuid>,
    country: Option<&str>,
    currency: Option<&str>,
    payment_method: Option<&str>,
    customer_type: Option<&str>,
    min_amount_minor: Option<i64>,
    max_amount_minor: Option<i64>,
) -> Result<(), Status> {
    let conflicting_rule_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT id
        FROM billing_provider_routing_rules
        WHERE status = 'active'
          AND ($1::uuid IS NULL OR id <> $1)
          AND (country IS NULL OR $2::text IS NULL OR country::text = $2)
          AND (currency IS NULL OR $3::text IS NULL OR currency::text = $3)
          AND (payment_method IS NULL OR $4::text IS NULL OR lower(payment_method) = lower($4))
          AND (customer_type IS NULL OR $5::text IS NULL OR lower(customer_type) = lower($5))
          AND COALESCE(min_amount_minor, 0) <= COALESCE($7, 9223372036854775807)
          AND COALESCE(max_amount_minor, 9223372036854775807) >= COALESCE($6, 0)
        ORDER BY priority ASC, updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(exclude_rule_id)
    .bind(country)
    .bind(currency)
    .bind(payment_method)
    .bind(customer_type)
    .bind(min_amount_minor)
    .bind(max_amount_minor)
    .fetch_optional(&mut *executor)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    reject_conflict(conflicting_rule_id)
}

pub async fn reject_active_routing_rule_overlap_for_rule(
    executor: &mut PgConnection,
    rule_id: Uuid,
) -> Result<(), Status> {
    let row = sqlx::query(
        r#"
        SELECT country::text AS country, currency::text AS currency, payment_method,
          customer_type, min_amount_minor, max_amount_minor
        FROM billing_provider_routing_rules
        WHERE id = $1
        "#,
    )
    .bind(rule_id)
    .fetch_optional(&mut *executor)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    let Some(row) = row else {
        return Ok(());
    };

    reject_active_routing_rule_overlap(
        executor,
        Some(rule_id),
        row.get::<Option<String>, _>("country").as_deref(),
        row.get::<Option<String>, _>("currency").as_deref(),
        row.get::<Option<String>, _>("payment_method").as_deref(),
        row.get::<Option<String>, _>("customer_type").as_deref(),
        row.get::<Option<i64>, _>("min_amount_minor"),
        row.get::<Option<i64>, _>("max_amount_minor"),
    )
    .await
}

pub fn normalize_code(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_ascii_uppercase())
}

pub fn normalize_text_filter(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_ascii_lowercase())
}

fn validate_code_len(value: &str, expected: usize, code: &'static str) -> Result<(), Status> {
    if value.trim().is_empty() || value.len() == expected {
        return Ok(());
    }
    Err(Status::invalid_argument(format!(
        "{code}: Routing filter code is invalid."
    )))
}

fn reject_conflict(conflicting_rule_id: Option<Uuid>) -> Result<(), Status> {
    if let Some(rule_id) = conflicting_rule_id {
        return Err(Status::failed_precondition(format!(
            "routing_rule_overlap: Routing rule overlaps active rule {rule_id}. Disable the existing rule first."
        )));
    }
    Ok(())
}

