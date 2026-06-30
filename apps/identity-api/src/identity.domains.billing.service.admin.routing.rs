use crate::http::error::AppError;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRoutingRuleInput {
    pub tenant_id: Uuid,
    pub workspace_id: Uuid,
    pub actor_principal_id: Uuid,
    pub provider: String,
    pub country: Option<String>,
    pub currency: Option<String>,
    pub payment_method: Option<String>,
    pub customer_type: Option<String>,
    pub min_amount_minor: Option<i64>,
    pub max_amount_minor: Option<i64>,
    pub fallback_enabled: bool,
    pub priority: i32,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisableProviderRoutingRuleInput {
    pub tenant_id: Uuid,
    pub workspace_id: Uuid,
    pub actor_principal_id: Uuid,
    pub routing_rule_id: Uuid,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ProviderRoutingRuleMutationResult {
    pub routing_rule_id: Uuid,
    pub audit_action: &'static str,
}

pub async fn create_provider_routing_rule(
    db: &sqlx::PgPool,
    input: ProviderRoutingRuleInput,
) -> Result<ProviderRoutingRuleMutationResult, AppError> {
    validate_provider_routing_rule(&input)?;
    let mut tx = db.begin().await?;
    let routing_rule_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO billing_provider_routing_rules (
          priority, provider, country, currency, payment_method, customer_type,
          min_amount_minor, max_amount_minor, fallback_enabled, status
        )
        VALUES ($1, $2::billing_provider, $3, $4, $5, $6, $7, $8, $9, 'active')
        RETURNING id
        "#,
    )
    .bind(input.priority)
    .bind(input.provider.as_str())
    .bind(normalize_country(input.country.as_deref()))
    .bind(normalize_currency(input.currency.as_deref()))
    .bind(input.payment_method.as_deref())
    .bind(input.customer_type.as_deref())
    .bind(input.min_amount_minor)
    .bind(input.max_amount_minor)
    .bind(input.fallback_enabled)
    .fetch_one(tx.as_mut())
    .await?;

    insert_billing_platform_action(
        &mut tx,
        BillingPlatformActionInput {
            tenant_id: input.tenant_id,
            workspace_id: input.workspace_id,
            actor_principal_id: input.actor_principal_id,
            action_kind: "provider_routing_rule.created",
            routing_rule_id,
            previous_state: None,
            next_state: "active",
            reason: &input.reason,
        },
    )
    .await?;
    tx.commit().await?;

    Ok(ProviderRoutingRuleMutationResult {
        routing_rule_id,
        audit_action: "provider_routing_rule.created",
    })
}

pub async fn disable_provider_routing_rule(
    db: &sqlx::PgPool,
    input: DisableProviderRoutingRuleInput,
) -> Result<ProviderRoutingRuleMutationResult, AppError> {
    validate_reason(&input.reason)?;
    let mut tx = db.begin().await?;
    let updated = sqlx::query(
        r#"
        UPDATE billing_provider_routing_rules
        SET status = 'disabled', updated_at = NOW()
        WHERE id = $1 AND status = 'active'
        "#,
    )
    .bind(input.routing_rule_id)
    .execute(tx.as_mut())
    .await?;

    if updated.rows_affected() == 0 {
        return Err(AppError::not_found(
            "routing_rule_not_found",
            "Active provider routing rule not found.",
        ));
    }

    insert_billing_platform_action(
        &mut tx,
        BillingPlatformActionInput {
            tenant_id: input.tenant_id,
            workspace_id: input.workspace_id,
            actor_principal_id: input.actor_principal_id,
            action_kind: "provider_routing_rule.disabled",
            routing_rule_id: input.routing_rule_id,
            previous_state: Some("active"),
            next_state: "disabled",
            reason: &input.reason,
        },
    )
    .await?;
    tx.commit().await?;

    Ok(ProviderRoutingRuleMutationResult {
        routing_rule_id: input.routing_rule_id,
        audit_action: "provider_routing_rule.disabled",
    })
}

struct BillingPlatformActionInput<'a> {
    tenant_id: Uuid,
    workspace_id: Uuid,
    actor_principal_id: Uuid,
    action_kind: &'a str,
    routing_rule_id: Uuid,
    previous_state: Option<&'a str>,
    next_state: &'a str,
    reason: &'a str,
}

async fn insert_billing_platform_action(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: BillingPlatformActionInput<'_>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO internal_admin_billing_platform_actions (
          tenant_id, workspace_id, actor_principal_id, action_kind, routing_rule_id,
          previous_state, next_state, reason
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(input.tenant_id)
    .bind(input.workspace_id)
    .bind(input.actor_principal_id)
    .bind(input.action_kind)
    .bind(input.routing_rule_id)
    .bind(input.previous_state)
    .bind(input.next_state)
    .bind(input.reason)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}

fn validate_provider_routing_rule(input: &ProviderRoutingRuleInput) -> Result<(), AppError> {
    validate_provider(&input.provider)?;
    validate_reason(&input.reason)?;
    if let Some(country) = &input.country
        && country.len() != 2
    {
        return Err(AppError::bad_request(
            "invalid_routing_country",
            "Routing country must be an ISO 3166-1 alpha-2 code.",
        ));
    }
    if let Some(currency) = &input.currency
        && currency.len() != 3
    {
        return Err(AppError::bad_request(
            "invalid_routing_currency",
            "Routing currency must be an ISO 4217 code.",
        ));
    }
    if input
        .min_amount_minor
        .is_some_and(|amount_minor| amount_minor < 0)
        || input
            .max_amount_minor
            .is_some_and(|amount_minor| amount_minor < 0)
    {
        return Err(AppError::bad_request(
            "invalid_routing_amount",
            "Routing amount bounds must be non-negative.",
        ));
    }
    if let (Some(min_amount), Some(max_amount)) = (input.min_amount_minor, input.max_amount_minor)
        && min_amount > max_amount
    {
        return Err(AppError::bad_request(
            "invalid_routing_amount_range",
            "Routing minimum amount cannot exceed maximum amount.",
        ));
    }
    Ok(())
}

fn validate_provider(provider: &str) -> Result<(), AppError> {
    if matches!(provider, "stripe" | "mollie") {
        Ok(())
    } else {
        Err(AppError::bad_request(
            "invalid_routing_provider",
            "Routing provider is not supported.",
        ))
    }
}

fn validate_reason(reason: &str) -> Result<(), AppError> {
    if reason.trim().len() >= 8 {
        Ok(())
    } else {
        Err(AppError::bad_request(
            "invalid_admin_reason",
            "Admin mutation reason must be at least 8 characters.",
        ))
    }
}

fn normalize_country(country: Option<&str>) -> Option<String> {
    country.map(str::to_ascii_uppercase)
}

fn normalize_currency(currency: Option<&str>) -> Option<String> {
    currency.map(str::to_ascii_uppercase)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_input() -> ProviderRoutingRuleInput {
        ProviderRoutingRuleInput {
            tenant_id: Uuid::nil(),
            workspace_id: Uuid::nil(),
            actor_principal_id: Uuid::nil(),
            provider: "mollie".to_string(),
            country: Some("fr".to_string()),
            currency: Some("eur".to_string()),
            payment_method: Some("card".to_string()),
            customer_type: Some("b2b".to_string()),
            min_amount_minor: Some(100),
            max_amount_minor: Some(10_000),
            fallback_enabled: false,
            priority: 10,
            reason: "prefer local PSP".to_string(),
        }
    }

    #[test]
    fn routing_rule_validation_rejects_unknown_provider() {
        let mut input = valid_input();
        input.provider = "unknown".to_string();
        assert!(validate_provider_routing_rule(&input).is_err());
    }

    #[test]
    fn routing_rule_validation_rejects_invalid_amount_range() {
        let mut input = valid_input();
        input.min_amount_minor = Some(2_000);
        input.max_amount_minor = Some(1_000);
        assert!(validate_provider_routing_rule(&input).is_err());
    }

    #[test]
    fn routing_rule_normalizes_country_and_currency() {
        assert_eq!(normalize_country(Some("fr")), Some("FR".to_string()));
        assert_eq!(normalize_currency(Some("eur")), Some("EUR".to_string()));
    }
}
