use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::billing_grpc_platform::{
    BackofficeBillingPlatformActionKind, BackofficeBillingPlatformActionOutcome,
    run_billing_platform_action,
};
use crate::billing_platform_center_action_log::{
    BillingPlatformActionInput, insert_action, insert_audit,
};
use crate::billing_platform_center_types::{BillingPlatformActionResult, action_result};
use crate::billing_platform_center_validation::validate_reason;
use crate::error::AppError;

pub(crate) async fn approve_kyc_profile(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    profile_id: Uuid,
    reason: String,
) -> Result<BillingPlatformActionResult, AppError> {
    review_kyc_profile(
        db,
        billing_grpc_endpoint,
        access,
        workspace_id,
        profile_id,
        BackofficeBillingPlatformActionKind::ApproveKycProfile,
        reason,
    )
    .await
}

pub(crate) async fn reject_kyc_profile(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    profile_id: Uuid,
    reason: String,
) -> Result<BillingPlatformActionResult, AppError> {
    review_kyc_profile(
        db,
        billing_grpc_endpoint,
        access,
        workspace_id,
        profile_id,
        BackofficeBillingPlatformActionKind::RejectKycProfile,
        reason,
    )
    .await
}

pub(crate) async fn activate_einvoicing_profile(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    profile_id: Uuid,
    reason: String,
) -> Result<BillingPlatformActionResult, AppError> {
    validate_reason(&reason)?;
    let outcome = run_billing_platform_action(
        billing_grpc_endpoint,
        access.tenant_id,
        workspace_id,
        access.actor_principal_id,
        BackofficeBillingPlatformActionKind::ActivateEinvoicingProfile,
        Some(profile_id),
        reason.clone(),
        None,
    )
    .await?;
    record_platform_action(
        db,
        access,
        workspace_id,
        reason,
        PlatformTarget::EinvoicingProfile(profile_id),
        outcome,
    )
    .await
}

async fn review_kyc_profile(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    profile_id: Uuid,
    action_kind: BackofficeBillingPlatformActionKind,
    reason: String,
) -> Result<BillingPlatformActionResult, AppError> {
    validate_reason(&reason)?;
    let outcome = run_billing_platform_action(
        billing_grpc_endpoint,
        access.tenant_id,
        workspace_id,
        access.actor_principal_id,
        action_kind,
        Some(profile_id),
        reason.clone(),
        None,
    )
    .await?;
    record_platform_action(
        db,
        access,
        workspace_id,
        reason,
        PlatformTarget::KycProfile(profile_id),
        outcome,
    )
    .await
}

pub(crate) enum PlatformTarget {
    KycProfile(Uuid),
    EinvoicingProfile(Uuid),
    RoutingRule(Uuid),
}

pub(crate) async fn record_platform_action(
    db: &PgPool,
    access: BackofficeAccess,
    workspace_id: Uuid,
    reason: String,
    target: PlatformTarget,
    outcome: BackofficeBillingPlatformActionOutcome,
) -> Result<BillingPlatformActionResult, AppError> {
    let action_kind = action_kind(&outcome.action_kind)?;
    let next_state = action_status(&outcome.status)?;
    let audit_action = audit_action(&outcome.audit_action)?;
    let target_type = target_type(&outcome.target_type)?;
    let metadata = action_metadata(workspace_id, &target, &outcome);
    let mut tx = db.begin().await?;
    let action_id = insert_action(
        &mut tx,
        access,
        workspace_id,
        BillingPlatformActionInput {
            action_kind,
            routing_rule_id: target_id(&target, "routing_rule"),
            kyc_profile_id: target_id(&target, "kyc_profile"),
            einvoicing_profile_id: target_id(&target, "einvoicing_profile"),
            previous_state: outcome.previous_state.clone(),
            next_state,
            reason,
            metadata: outcome.metadata,
        },
    )
    .await?;
    insert_audit(
        &mut tx,
        access,
        workspace_id,
        audit_action,
        target_type,
        outcome.object_id,
        action_id,
        metadata,
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

fn target_id(target: &PlatformTarget, field: &str) -> Option<Uuid> {
    match (target, field) {
        (PlatformTarget::KycProfile(id), "kyc_profile") => Some(*id),
        (PlatformTarget::EinvoicingProfile(id), "einvoicing_profile") => Some(*id),
        (PlatformTarget::RoutingRule(id), "routing_rule") => Some(*id),
        _ => None,
    }
}

fn action_metadata(
    workspace_id: Uuid,
    target: &PlatformTarget,
    outcome: &BackofficeBillingPlatformActionOutcome,
) -> Value {
    let previous_state = outcome.previous_state.as_deref();
    match target {
        PlatformTarget::KycProfile(id) => json!({
            "object_links": {
                "kyc_profile_id": id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "review_status",
                    "before": previous_state,
                    "after": outcome.status,
                },
                {
                    "field": "reviewed_by_principal_id",
                    "before": null,
                    "after": "recorded",
                },
                {
                    "field": "review_reason",
                    "before": null,
                    "after": "recorded",
                }
            ],
        }),
        PlatformTarget::EinvoicingProfile(id) => json!({
            "object_links": {
                "einvoicing_profile_id": id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "status",
                    "before": previous_state,
                    "after": outcome.status,
                }
            ],
        }),
        PlatformTarget::RoutingRule(id) => json!({
            "object_links": {
                "routing_rule_id": id,
                "workspace_id": workspace_id,
            },
            "changes": [
                {
                    "field": "status",
                    "before": previous_state,
                    "after": outcome.status,
                }
            ],
        }),
    }
}

fn action_kind(value: &str) -> Result<&'static str, AppError> {
    match value {
        "approve_kyc_profile" => Ok("approve_kyc_profile"),
        "reject_kyc_profile" => Ok("reject_kyc_profile"),
        "activate_einvoicing_profile" => Ok("activate_einvoicing_profile"),
        "create_routing_rule" => Ok("create_routing_rule"),
        "enable_routing_rule" => Ok("enable_routing_rule"),
        "disable_routing_rule" => Ok("disable_routing_rule"),
        _ => Err(AppError::internal(
            "billing_grpc_invalid_action_kind",
            format!("unexpected billing platform action kind: {value}"),
        )),
    }
}

fn action_status(value: &str) -> Result<&'static str, AppError> {
    match value {
        "approved" => Ok("approved"),
        "rejected" => Ok("rejected"),
        "active" => Ok("active"),
        "disabled" => Ok("disabled"),
        _ => Err(AppError::internal(
            "billing_grpc_invalid_action_status",
            format!("unexpected billing platform action status: {value}"),
        )),
    }
}

fn audit_action(value: &str) -> Result<&'static str, AppError> {
    match value {
        "billing_platform.kyc.approved" => Ok("billing_platform.kyc.approved"),
        "billing_platform.kyc.rejected" => Ok("billing_platform.kyc.rejected"),
        "billing_platform.einvoicing_profile.activated" => {
            Ok("billing_platform.einvoicing_profile.activated")
        }
        "billing_platform.routing_rule.created" => Ok("billing_platform.routing_rule.created"),
        "billing_platform.routing_rule.enabled" => Ok("billing_platform.routing_rule.enabled"),
        "billing_platform.routing_rule.disabled" => Ok("billing_platform.routing_rule.disabled"),
        _ => Err(AppError::internal(
            "billing_grpc_invalid_audit_action",
            format!("unexpected billing platform audit action: {value}"),
        )),
    }
}

fn target_type(value: &str) -> Result<&'static str, AppError> {
    match value {
        "billing_kyc_profile" => Ok("billing_kyc_profile"),
        "billing_einvoicing_profile" => Ok("billing_einvoicing_profile"),
        "billing_provider_routing_rule" => Ok("billing_provider_routing_rule"),
        _ => Err(AppError::internal(
            "billing_grpc_invalid_target_type",
            format!("unexpected billing platform target type: {value}"),
        )),
    }
}
