use uuid::Uuid;

use crate::{
    billing_grpc::{
        billing_client, empty_to_none, grpc_error, parse_uuid, request_context_for_workspace,
    },
    error::AppError,
    grpc_pb::nvbes::billing::v1::{
        AdminOperationsActionKind, AdminOperationsActionRequest, AdminOperationsActionResult,
    },
};

pub struct BackofficeOperationsActionOutcome {
    pub object_id: Uuid,
    pub action_kind: String,
    pub status: String,
    pub audit_action: String,
    pub target_type: String,
    pub previous_state: Option<String>,
}

pub enum BackofficeOperationsActionKind {
    ReplayProviderEvent,
    ReplayExportRun,
    ResolveReconciliationDifference,
}

pub async fn run_admin_operations_action(
    endpoint: &str,
    tenant_id: Uuid,
    workspace_id: Uuid,
    actor_id: Uuid,
    action_kind: BackofficeOperationsActionKind,
    target_id: Uuid,
    reason: String,
) -> Result<BackofficeOperationsActionOutcome, AppError> {
    let mut client = billing_client(endpoint).await?;
    let response = client
        .run_admin_operations_action(AdminOperationsActionRequest {
            context: Some(request_context_for_workspace(
                tenant_id,
                workspace_id,
                actor_id,
            )),
            workspace_id: workspace_id.to_string(),
            action_kind: admin_operations_action_kind(action_kind) as i32,
            target_id: target_id.to_string(),
            reason,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();
    operations_action_outcome(response)
}

fn operations_action_outcome(
    value: AdminOperationsActionResult,
) -> Result<BackofficeOperationsActionOutcome, AppError> {
    Ok(BackofficeOperationsActionOutcome {
        object_id: parse_uuid(&value.object_id)?,
        action_kind: value.action_kind,
        status: value.status,
        audit_action: value.audit_action,
        target_type: value.target_type,
        previous_state: empty_to_none(value.previous_state),
    })
}

fn admin_operations_action_kind(
    value: BackofficeOperationsActionKind,
) -> AdminOperationsActionKind {
    match value {
        BackofficeOperationsActionKind::ReplayProviderEvent => {
            AdminOperationsActionKind::ReplayProviderEvent
        }
        BackofficeOperationsActionKind::ReplayExportRun => {
            AdminOperationsActionKind::ReplayExportRun
        }
        BackofficeOperationsActionKind::ResolveReconciliationDifference => {
            AdminOperationsActionKind::ResolveReconciliationDifference
        }
    }
}
