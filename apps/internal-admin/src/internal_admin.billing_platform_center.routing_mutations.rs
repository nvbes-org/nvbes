use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::billing_platform_center_action_log::{
    BillingPlatformActionInput, insert_action, insert_audit,
};
use crate::billing_platform_center_routing_rule_conflicts::{
    RoutingRuleConflictInput, reject_active_routing_rule_overlap,
    reject_active_routing_rule_overlap_for_rule,
};
use crate::billing_platform_center_routing_rule_validation::{
    normalize_country, normalize_currency, normalize_text_filter, validate_create_routing_rule,
};
use crate::billing_platform_center_types::{BillingPlatformActionResult, action_result};
use crate::billing_platform_center_validation::validate_reason;
use crate::error::AppError;

pub(crate) struct CreateRoutingRuleInput {
    pub(crate) provider: String,
    pub(crate) country: Option<String>,
    pub(crate) currency: Option<String>,
    pub(crate) payment_method: Option<String>,
    pub(crate) customer_type: Option<String>,
    pub(crate) min_amount_minor: Option<i64>,
    pub(crate) max_amount_minor: Option<i64>,
    pub(crate) fallback_enabled: bool,
    pub(crate) priority: i32,
    pub(crate) reason: String,
}

pub(crate) async fn create_routing_rule(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    input: CreateRoutingRuleInput,
) -> Result<BillingPlatformActionResult, AppError> {
    validate_create_routing_rule(&input)?;
    let country = normalize_country(input.country.as_deref());
    let currency = normalize_currency(input.currency.as_deref());
    let payment_method = normalize_text_filter(input.payment_method.as_deref());
    let customer_type = normalize_text_filter(input.customer_type.as_deref());
    let mut tx = db.begin().await?;
    reject_active_routing_rule_overlap(
        &mut tx,
        RoutingRuleConflictInput {
            exclude_rule_id: None,
            country: country.as_deref(),
            currency: currency.as_deref(),
            payment_method: payment_method.as_deref(),
            customer_type: customer_type.as_deref(),
            min_amount_minor: input.min_amount_minor,
            max_amount_minor: input.max_amount_minor,
        },
    )
    .await?;
    let rule_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO billing_provider_routing_rules (
           priority, provider, country, currency, payment_method, customer_type,
           min_amount_minor, max_amount_minor, fallback_enabled, status
         ) VALUES ($1, $2::billing_provider, $3, $4, $5, $6, $7, $8, $9, 'active')
         RETURNING id",
    )
    .bind(input.priority)
    .bind(input.provider.as_str())
    .bind(country.as_deref())
    .bind(currency.as_deref())
    .bind(payment_method.as_deref())
    .bind(customer_type.as_deref())
    .bind(input.min_amount_minor)
    .bind(input.max_amount_minor)
    .bind(input.fallback_enabled)
    .fetch_one(tx.as_mut())
    .await?;
    let metadata = json!({
        "routing_rule_id": rule_id,
        "provider": input.provider,
        "country": country,
        "currency": currency,
        "payment_method": payment_method,
        "customer_type": customer_type,
        "min_amount_minor": input.min_amount_minor,
        "max_amount_minor": input.max_amount_minor,
        "fallback_enabled": input.fallback_enabled,
        "priority": input.priority,
    });
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        BillingPlatformActionInput {
            action_kind: "create_routing_rule",
            routing_rule_id: Some(rule_id),
            kyc_profile_id: None,
            einvoicing_profile_id: None,
            previous_state: None,
            next_state: "active",
            reason: input.reason,
            metadata: metadata.clone(),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        "billing_platform.routing_rule.created",
        "billing_provider_routing_rule",
        rule_id,
        action_id,
        json!({
            "object_links": {
                "routing_rule_id": rule_id,
                "workspace_id": workspace_id,
            },
            "created": metadata,
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "create_routing_rule",
        "active",
        "billing_platform.routing_rule.created",
    ))
}

pub(crate) async fn enable_routing_rule(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    rule_id: Uuid,
    reason: String,
) -> Result<BillingPlatformActionResult, AppError> {
    transition_routing_rule(db, access, workspace_id, rule_id, "active", reason).await
}

pub(crate) async fn disable_routing_rule(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    rule_id: Uuid,
    reason: String,
) -> Result<BillingPlatformActionResult, AppError> {
    transition_routing_rule(db, access, workspace_id, rule_id, "disabled", reason).await
}

async fn transition_routing_rule(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    rule_id: Uuid,
    next_state: &'static str,
    reason: String,
) -> Result<BillingPlatformActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    if next_state == "active" {
        reject_active_routing_rule_overlap_for_rule(&mut tx, rule_id).await?;
    }
    let row = sqlx::query(
        "WITH previous AS (
           SELECT id, status AS previous_state
           FROM billing_provider_routing_rules
           WHERE id = $1 AND status <> $2
         ),
         updated AS (
           UPDATE billing_provider_routing_rules bprr
           SET status = $2, updated_at = NOW()
           FROM previous
           WHERE bprr.id = previous.id
           RETURNING bprr.status AS next_state
         )
         SELECT previous.previous_state, updated.next_state
         FROM previous
         JOIN updated ON TRUE",
    )
    .bind(rule_id)
    .bind(next_state)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "routing_rule_not_transitionable",
            "Routing rule is missing or already has the requested status.",
        )
    })?;
    let (action_kind, audit_action) = routing_action_names(next_state);
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        BillingPlatformActionInput {
            action_kind,
            routing_rule_id: Some(rule_id),
            kyc_profile_id: None,
            einvoicing_profile_id: None,
            previous_state: Some(row.get("previous_state")),
            next_state,
            reason,
            metadata: json!({ "routing_rule_id": rule_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        audit_action,
        "billing_provider_routing_rule",
        rule_id,
        action_id,
        json!({
            "object_links": {
                "routing_rule_id": rule_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "status",
                    "before": row.get::<String, _>("previous_state"),
                    "after": next_state,
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        action_kind,
        next_state,
        audit_action,
    ))
}

fn routing_action_names(next_state: &'static str) -> (&'static str, &'static str) {
    if next_state == "active" {
        (
            "enable_routing_rule",
            "billing_platform.routing_rule.enabled",
        )
    } else {
        (
            "disable_routing_rule",
            "billing_platform.routing_rule.disabled",
        )
    }
}
