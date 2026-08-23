use serde_json::Value;
use uuid::Uuid;

use crate::{
    billing_grpc::{
        billing_client, empty_to_none, grpc_error, parse_uuid, request_context_for_workspace,
    },
    error::AppError,
    grpc_pb::nvbes::billing::v1::{
        AdminBillingPlatformActionKind, AdminBillingPlatformActionRequest,
        AdminBillingPlatformActionResult, CreateBillingRoutingRuleInput,
    },
};

pub struct BackofficeBillingPlatformActionOutcome {
    pub object_id: Uuid,
    pub action_kind: String,
    pub status: String,
    pub audit_action: String,
    pub target_type: String,
    pub previous_state: Option<String>,
    pub metadata: Value,
}

pub enum BackofficeBillingPlatformActionKind {
    ApproveKycProfile,
    RejectKycProfile,
    ActivateEinvoicingProfile,
    CreateRoutingRule,
    EnableRoutingRule,
    DisableRoutingRule,
}

pub async fn run_billing_platform_action(
    endpoint: &str,
    tenant_id: Uuid,
    workspace_id: Uuid,
    actor_id: Uuid,
    action_kind: BackofficeBillingPlatformActionKind,
    target_id: Option<Uuid>,
    reason: String,
    routing_rule: Option<CreateBillingRoutingRuleInput>,
) -> Result<BackofficeBillingPlatformActionOutcome, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .run_admin_billing_platform_action(AdminBillingPlatformActionRequest {
            context: Some(request_context_for_workspace(
                tenant_id,
                workspace_id,
                actor_id,
            )),
            workspace_id: workspace_id.to_string(),
            action_kind: billing_platform_action_kind(action_kind) as i32,
            target_id: target_id.map_or_else(String::new, |id| id.to_string()),
            reason,
            routing_rule,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();
    billing_platform_outcome(response)
}

fn billing_platform_outcome(
    value: AdminBillingPlatformActionResult,
) -> Result<BackofficeBillingPlatformActionOutcome, AppError> {
    Ok(BackofficeBillingPlatformActionOutcome {
        object_id: parse_uuid(&value.object_id)?,
        action_kind: value.action_kind,
        status: value.status,
        audit_action: value.audit_action,
        target_type: value.target_type,
        previous_state: empty_to_none(value.previous_state),
        metadata: parse_metadata(value.metadata_json)?,
    })
}

fn parse_metadata(value: String) -> Result<Value, AppError> {
    empty_to_none(value).map_or(Ok(Value::Null), |value| {
        serde_json::from_str(&value)
            .map_err(|error| AppError::internal("billing_grpc_invalid_metadata", error.to_string()))
    })
}

fn billing_platform_action_kind(
    value: BackofficeBillingPlatformActionKind,
) -> AdminBillingPlatformActionKind {
    match value {
        BackofficeBillingPlatformActionKind::ApproveKycProfile => {
            AdminBillingPlatformActionKind::ApproveKycProfile
        }
        BackofficeBillingPlatformActionKind::RejectKycProfile => {
            AdminBillingPlatformActionKind::RejectKycProfile
        }
        BackofficeBillingPlatformActionKind::ActivateEinvoicingProfile => {
            AdminBillingPlatformActionKind::ActivateEinvoicingProfile
        }
        BackofficeBillingPlatformActionKind::CreateRoutingRule => {
            AdminBillingPlatformActionKind::CreateRoutingRule
        }
        BackofficeBillingPlatformActionKind::EnableRoutingRule => {
            AdminBillingPlatformActionKind::EnableRoutingRule
        }
        BackofficeBillingPlatformActionKind::DisableRoutingRule => {
            AdminBillingPlatformActionKind::DisableRoutingRule
        }
    }
}
