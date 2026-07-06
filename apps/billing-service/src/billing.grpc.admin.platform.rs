use serde_json::json;
use sqlx::Row;
use tonic::Status;
use uuid::Uuid;

use crate::grpc::pb::nvbes::billing::v1::{
    AdminBillingPlatformActionKind, AdminBillingPlatformActionResult, CreateBillingRoutingRuleInput,
};

pub async fn run_platform_action(
    db: &sqlx::PgPool,
    kind: AdminBillingPlatformActionKind,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    target_id: Option<Uuid>,
    reason: String,
    routing_rule: Option<CreateBillingRoutingRuleInput>,
) -> Result<AdminBillingPlatformActionResult, Status> {
    validate_reason(&reason)?;
    match kind {
        AdminBillingPlatformActionKind::ApproveKycProfile => {
            transition_kyc_profile(
                db,
                tenant_id,
                actor_principal_id,
                required_target_id(target_id)?,
                "approved",
                reason,
            )
            .await
        }
        AdminBillingPlatformActionKind::RejectKycProfile => {
            transition_kyc_profile(
                db,
                tenant_id,
                actor_principal_id,
                required_target_id(target_id)?,
                "rejected",
                reason,
            )
            .await
        }
        AdminBillingPlatformActionKind::ActivateEinvoicingProfile => {
            activate_einvoicing_profile(db, required_target_id(target_id)?).await
        }
        AdminBillingPlatformActionKind::CreateRoutingRule => {
            create_routing_rule(db, required_routing_rule(routing_rule)?).await
        }
        AdminBillingPlatformActionKind::EnableRoutingRule => {
            transition_routing_rule(db, required_target_id(target_id)?, "active").await
        }
        AdminBillingPlatformActionKind::DisableRoutingRule => {
            transition_routing_rule(db, required_target_id(target_id)?, "disabled").await
        }
        AdminBillingPlatformActionKind::ApproveFraudAssessment => {
            crate::grpc::service_admin_platform_fraud::review_fraud_assessment(
                db,
                tenant_id,
                actor_principal_id,
                required_target_id(target_id)?,
                "approved",
                reason,
            )
            .await
        }
        AdminBillingPlatformActionKind::RejectFraudAssessment => {
            crate::grpc::service_admin_platform_fraud::review_fraud_assessment(
                db,
                tenant_id,
                actor_principal_id,
                required_target_id(target_id)?,
                "rejected",
                reason,
            )
            .await
        }
        AdminBillingPlatformActionKind::TrustFraudAssessment => {
            crate::grpc::service_admin_platform_fraud::review_fraud_assessment(
                db,
                tenant_id,
                actor_principal_id,
                required_target_id(target_id)?,
                "trusted",
                reason,
            )
            .await
        }
        AdminBillingPlatformActionKind::Unspecified => Err(Status::invalid_argument(
            "billing platform action kind is required",
        )),
    }
}

fn required_target_id(target_id: Option<Uuid>) -> Result<Uuid, Status> {
    target_id.ok_or_else(|| Status::invalid_argument("target_id is required"))
}

async fn transition_kyc_profile(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    profile_id: Uuid,
    next_state: &'static str,
    reason: String,
) -> Result<AdminBillingPlatformActionResult, Status> {
    let row = sqlx::query_as::<_, (String, String)>(
        r#"
        WITH previous AS (
          SELECT id, review_status AS previous_state
          FROM billing_kyc_profiles
          WHERE id = $1 AND tenant_id = $2 AND review_status <> $3
        ),
        updated AS (
          UPDATE billing_kyc_profiles bkp
          SET review_status = $3, reviewed_at = NOW(), reviewed_by_principal_id = $4,
            review_reason = $5, updated_at = NOW()
          FROM previous
          WHERE bkp.id = previous.id
          RETURNING bkp.review_status AS next_state
        )
        SELECT previous.previous_state, updated.next_state
        FROM previous
        JOIN updated ON TRUE
        "#,
    )
    .bind(profile_id)
    .bind(tenant_id)
    .bind(next_state)
    .bind(actor_principal_id)
    .bind(reason)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition(
            "kyc_profile_not_reviewable: KYC profile is missing, belongs to another tenant, or already has the requested status.",
        )
    })?;
    let (action_kind, audit_action) = kyc_action_names(next_state);

    Ok(AdminBillingPlatformActionResult {
        object_id: profile_id.to_string(),
        action_kind: action_kind.to_string(),
        status: row.1,
        audit_action: audit_action.to_string(),
        target_type: "billing_kyc_profile".to_string(),
        previous_state: row.0,
        metadata_json: json!({ "kyc_profile_id": profile_id }).to_string(),
    })
}

async fn activate_einvoicing_profile(
    db: &sqlx::PgPool,
    profile_id: Uuid,
) -> Result<AdminBillingPlatformActionResult, Status> {
    let row = sqlx::query_as::<_, (String, String)>(
        r#"
        WITH previous AS (
          SELECT id, status AS previous_state
          FROM billing_einvoicing_profiles
          WHERE id = $1 AND status <> 'active'
        ),
        updated AS (
          UPDATE billing_einvoicing_profiles bep
          SET status = 'active', updated_at = NOW()
          FROM previous
          WHERE bep.id = previous.id
          RETURNING bep.status AS next_state
        )
        SELECT previous.previous_state, updated.next_state
        FROM previous
        JOIN updated ON TRUE
        "#,
    )
    .bind(profile_id)
    .fetch_optional(db)
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition(
            "einvoicing_profile_not_activatable: E-invoicing profile is missing or already active.",
        )
    })?;

    Ok(AdminBillingPlatformActionResult {
        object_id: profile_id.to_string(),
        action_kind: "activate_einvoicing_profile".to_string(),
        status: row.1,
        audit_action: "billing_platform.einvoicing_profile.activated".to_string(),
        target_type: "billing_einvoicing_profile".to_string(),
        previous_state: row.0,
        metadata_json: json!({ "einvoicing_profile_id": profile_id }).to_string(),
    })
}

async fn create_routing_rule(
    db: &sqlx::PgPool,
    input: CreateBillingRoutingRuleInput,
) -> Result<AdminBillingPlatformActionResult, Status> {
    validate_routing_rule(&input)?;
    let country = normalize_code(&input.country);
    let currency = normalize_code(&input.currency);
    let payment_method = normalize_text_filter(&input.payment_method);
    let customer_type = normalize_text_filter(&input.customer_type);
    let min_amount_minor = input.has_min_amount_minor.then_some(input.min_amount_minor);
    let max_amount_minor = input.has_max_amount_minor.then_some(input.max_amount_minor);

    let mut tx = db
        .begin()
        .await
        .map_err(crate::grpc::service_status::sql_status)?;
    reject_active_routing_rule_overlap(
        tx.as_mut(),
        None,
        country.as_deref(),
        currency.as_deref(),
        payment_method.as_deref(),
        customer_type.as_deref(),
        min_amount_minor,
        max_amount_minor,
    )
    .await?;
    let rule_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO billing_provider_routing_rules (
          priority, provider, country, currency, payment_method, customer_type,
          min_amount_minor, max_amount_minor, fallback_enabled, status
        ) VALUES ($1, $2::billing_provider, $3, $4, $5, $6, $7, $8, $9, 'active')
        RETURNING id
        "#,
    )
    .bind(input.priority)
    .bind(input.provider.as_str())
    .bind(country.as_deref())
    .bind(currency.as_deref())
    .bind(payment_method.as_deref())
    .bind(customer_type.as_deref())
    .bind(min_amount_minor)
    .bind(max_amount_minor)
    .bind(input.fallback_enabled)
    .fetch_one(tx.as_mut())
    .await
    .map_err(crate::grpc::service_status::sql_status)?;
    tx.commit()
        .await
        .map_err(crate::grpc::service_status::sql_status)?;

    let metadata = json!({
        "routing_rule_id": rule_id,
        "provider": input.provider,
        "country": country,
        "currency": currency,
        "payment_method": payment_method,
        "customer_type": customer_type,
        "min_amount_minor": min_amount_minor,
        "max_amount_minor": max_amount_minor,
        "fallback_enabled": input.fallback_enabled,
        "priority": input.priority,
    });

    Ok(AdminBillingPlatformActionResult {
        object_id: rule_id.to_string(),
        action_kind: "create_routing_rule".to_string(),
        status: "active".to_string(),
        audit_action: "billing_platform.routing_rule.created".to_string(),
        target_type: "billing_provider_routing_rule".to_string(),
        previous_state: String::new(),
        metadata_json: metadata.to_string(),
    })
}

async fn transition_routing_rule(
    db: &sqlx::PgPool,
    rule_id: Uuid,
    next_state: &'static str,
) -> Result<AdminBillingPlatformActionResult, Status> {
    let mut tx = db
        .begin()
        .await
        .map_err(crate::grpc::service_status::sql_status)?;
    if next_state == "active" {
        reject_active_routing_rule_overlap_for_rule(tx.as_mut(), rule_id).await?;
    }
    let row = sqlx::query_as::<_, (String, String)>(
        r#"
        WITH previous AS (
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
        JOIN updated ON TRUE
        "#,
    )
    .bind(rule_id)
    .bind(next_state)
    .fetch_optional(tx.as_mut())
    .await
    .map_err(crate::grpc::service_status::sql_status)?
    .ok_or_else(|| {
        Status::failed_precondition(
            "routing_rule_not_transitionable: Routing rule is missing or already has the requested status.",
        )
    })?;
    tx.commit()
        .await
        .map_err(crate::grpc::service_status::sql_status)?;
    let (action_kind, audit_action) = routing_action_names(next_state);

    Ok(AdminBillingPlatformActionResult {
        object_id: rule_id.to_string(),
        action_kind: action_kind.to_string(),
        status: row.1,
        audit_action: audit_action.to_string(),
        target_type: "billing_provider_routing_rule".to_string(),
        previous_state: row.0,
        metadata_json: json!({ "routing_rule_id": rule_id }).to_string(),
    })
}

async fn reject_active_routing_rule_overlap(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
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
    .fetch_optional(executor)
    .await
    .map_err(crate::grpc::service_status::sql_status)?;

    reject_conflict(conflicting_rule_id)
}

async fn reject_active_routing_rule_overlap_for_rule(
    executor: &mut sqlx::PgConnection,
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

fn validate_routing_rule(input: &CreateBillingRoutingRuleInput) -> Result<(), Status> {
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

fn validate_reason(value: &str) -> Result<(), Status> {
    let len = value.trim().len();
    if (8..=500).contains(&len) {
        return Ok(());
    }
    Err(Status::invalid_argument(
        "invalid_billing_platform_reason: Billing platform action reason must contain between 8 and 500 characters.",
    ))
}

fn validate_code_len(value: &str, expected: usize, code: &'static str) -> Result<(), Status> {
    if value.trim().is_empty() || value.len() == expected {
        return Ok(());
    }
    Err(Status::invalid_argument(format!(
        "{code}: Routing filter code is invalid."
    )))
}

fn normalize_code(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_ascii_uppercase())
}

fn normalize_text_filter(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_ascii_lowercase())
}

fn required_routing_rule(
    input: Option<CreateBillingRoutingRuleInput>,
) -> Result<CreateBillingRoutingRuleInput, Status> {
    input.ok_or_else(|| Status::invalid_argument("routing_rule input is required"))
}

fn reject_conflict(conflicting_rule_id: Option<Uuid>) -> Result<(), Status> {
    if let Some(rule_id) = conflicting_rule_id {
        return Err(Status::failed_precondition(format!(
            "routing_rule_overlap: Routing rule overlaps active rule {rule_id}. Disable the existing rule first."
        )));
    }
    Ok(())
}

fn kyc_action_names(next_state: &'static str) -> (&'static str, &'static str) {
    if next_state == "approved" {
        ("approve_kyc_profile", "billing_platform.kyc.approved")
    } else {
        ("reject_kyc_profile", "billing_platform.kyc.rejected")
    }
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
