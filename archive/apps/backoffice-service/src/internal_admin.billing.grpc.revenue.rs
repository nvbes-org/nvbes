use serde_json::Value;
use uuid::Uuid;

use crate::{
    billing_grpc::{
        billing_client, empty_to_none, grpc_error, parse_uuid, request_context_for_workspace,
    },
    error::AppError,
    grpc_pb::nvbes::billing::v1::{
        AdminRevenueActionKind, AdminRevenueActionRequest, AdminRevenueActionResult,
    },
};

pub struct BackofficeRevenueActionOutcome {
    pub object_id: Uuid,
    pub action_kind: String,
    pub status: String,
    pub audit_action: String,
    pub target_type: String,
    pub previous_state: Option<String>,
    pub metadata: Value,
}

pub enum BackofficeRevenueActionKind {
    CloseDunningCase,
    ReopenDunningCase,
    HoldInvoice,
    ReleaseInvoice,
    ReviewDispute,
    ResolveDispute,
}

pub async fn run_revenue_grpc_action(
    endpoint: &str,
    tenant_id: Uuid,
    workspace_id: Uuid,
    actor_id: Uuid,
    action_kind: BackofficeRevenueActionKind,
    target_id: Uuid,
    reason: String,
) -> Result<BackofficeRevenueActionOutcome, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .run_admin_revenue_action(AdminRevenueActionRequest {
            context: Some(request_context_for_workspace(
                tenant_id,
                workspace_id,
                actor_id,
            )),
            workspace_id: workspace_id.to_string(),
            action_kind: revenue_action_kind(action_kind) as i32,
            target_id: target_id.to_string(),
            reason,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();
    revenue_outcome(response)
}

fn revenue_outcome(
    value: AdminRevenueActionResult,
) -> Result<BackofficeRevenueActionOutcome, AppError> {
    Ok(BackofficeRevenueActionOutcome {
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

fn revenue_action_kind(value: BackofficeRevenueActionKind) -> AdminRevenueActionKind {
    match value {
        BackofficeRevenueActionKind::CloseDunningCase => AdminRevenueActionKind::CloseDunningCase,
        BackofficeRevenueActionKind::ReopenDunningCase => AdminRevenueActionKind::ReopenDunningCase,
        BackofficeRevenueActionKind::HoldInvoice => AdminRevenueActionKind::HoldInvoice,
        BackofficeRevenueActionKind::ReleaseInvoice => AdminRevenueActionKind::ReleaseInvoice,
        BackofficeRevenueActionKind::ReviewDispute => AdminRevenueActionKind::ReviewDispute,
        BackofficeRevenueActionKind::ResolveDispute => AdminRevenueActionKind::ResolveDispute,
    }
}
