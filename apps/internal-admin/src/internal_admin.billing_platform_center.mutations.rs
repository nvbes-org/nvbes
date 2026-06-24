use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::billing_platform_center_action_log::{
    BillingPlatformActionInput, insert_action, insert_audit,
};
use crate::billing_platform_center_types::{BillingPlatformActionResult, action_result};
use crate::billing_platform_center_validation::validate_reason;
use crate::error::AppError;

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

pub(crate) async fn approve_kyc_profile(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    profile_id: Uuid,
    reason: String,
) -> Result<BillingPlatformActionResult, AppError> {
    transition_kyc_profile(db, access, workspace_id, profile_id, "approved", reason).await
}

pub(crate) async fn reject_kyc_profile(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    profile_id: Uuid,
    reason: String,
) -> Result<BillingPlatformActionResult, AppError> {
    transition_kyc_profile(db, access, workspace_id, profile_id, "rejected", reason).await
}

pub(crate) async fn activate_einvoicing_profile(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    profile_id: Uuid,
    reason: String,
) -> Result<BillingPlatformActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "WITH previous AS (
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
         JOIN updated ON TRUE",
    )
    .bind(profile_id)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "einvoicing_profile_not_activatable",
            "E-invoicing profile is missing or already active.",
        )
    })?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        BillingPlatformActionInput {
            action_kind: "activate_einvoicing_profile",
            routing_rule_id: None,
            kyc_profile_id: None,
            einvoicing_profile_id: Some(profile_id),
            previous_state: Some(row.get("previous_state")),
            next_state: "active",
            reason,
            metadata: json!({ "einvoicing_profile_id": profile_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        "billing_platform.einvoicing_profile.activated",
        "billing_einvoicing_profile",
        profile_id,
        action_id,
        json!({
            "object_links": {
                "einvoicing_profile_id": profile_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "status",
                    "before": row.get::<String, _>("previous_state"),
                    "after": "active",
                }
            ],
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(action_result(
        action_id,
        "activate_einvoicing_profile",
        "active",
        "billing_platform.einvoicing_profile.activated",
    ))
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

async fn transition_kyc_profile(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    profile_id: Uuid,
    next_state: &'static str,
    reason: String,
) -> Result<BillingPlatformActionResult, AppError> {
    validate_reason(&reason)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "WITH previous AS (
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
         JOIN updated ON TRUE",
    )
    .bind(profile_id)
    .bind(access.tenant_id)
    .bind(next_state)
    .bind(access.actor_principal_id)
    .bind(&reason)
    .fetch_optional(tx.as_mut())
    .await?
    .ok_or_else(|| {
        AppError::conflict(
            "kyc_profile_not_reviewable",
            "KYC profile is missing, belongs to another tenant, or already has the requested status.",
        )
    })?;
    let (action_kind, audit_action) = kyc_action_names(next_state);
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        BillingPlatformActionInput {
            action_kind,
            routing_rule_id: None,
            kyc_profile_id: Some(profile_id),
            einvoicing_profile_id: None,
            previous_state: Some(row.get("previous_state")),
            next_state,
            reason,
            metadata: json!({ "kyc_profile_id": profile_id }),
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        audit_action,
        "billing_kyc_profile",
        profile_id,
        action_id,
        json!({
            "object_links": {
                "kyc_profile_id": profile_id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "review_status",
                    "before": row.get::<String, _>("previous_state"),
                    "after": next_state,
                },
                {
                    "field": "reviewed_by_principal_id",
                    "before": null,
                    "after": access.actor_principal_id,
                },
                {
                    "field": "review_reason",
                    "before": null,
                    "after": "recorded",
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

fn kyc_action_names(next_state: &'static str) -> (&'static str, &'static str) {
    if next_state == "approved" {
        ("approve_kyc_profile", "billing_platform.kyc.approved")
    } else {
        ("reject_kyc_profile", "billing_platform.kyc.rejected")
    }
}
