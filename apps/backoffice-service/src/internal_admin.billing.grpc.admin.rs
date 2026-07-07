use uuid::Uuid;

use crate::{
    billing_grpc::{billing_client, grpc_error, parse_uuid, request_context_for_workspace},
    error::AppError,
    grpc_pb::nvbes::billing::v1::{
        AdminBillingActionKind, AdminBillingActionRequest, AdminBillingActionResult,
    },
};

pub struct BackofficeBillingAdminActionOutcome {
    pub object_id: Uuid,
    pub ledger_entry_count: u64,
    pub audit_action: String,
    pub target_type: String,
    pub provider: String,
    pub provider_event_id: String,
    pub status: String,
}

pub enum BackofficeBillingAdminActionKind {
    CreateCreditNote,
    CreateWriteOff,
    CreateRefundIntent,
    CreateManualCompensation,
    ReplayProviderEvent,
    CreateProviderMigration,
    OverrideGracePeriod,
}

pub async fn run_billing_admin_action(
    endpoint: &str,
    tenant_id: Uuid,
    workspace_id: Uuid,
    actor_id: Uuid,
    action_kind: BackofficeBillingAdminActionKind,
    mut request: AdminBillingActionRequest,
) -> Result<BackofficeBillingAdminActionOutcome, AppError> {
    let mut client = billing_client(endpoint).await?;
    request.context = Some(request_context_for_workspace(
        tenant_id,
        workspace_id,
        actor_id,
    ));
    request.workspace_id = workspace_id.to_string();
    request.action_kind = billing_admin_action_kind(action_kind) as i32;
    let response = client
        .run_admin_billing_action(request)
        .await
        .map_err(grpc_error)?
        .into_inner();
    billing_admin_outcome(response)
}

fn billing_admin_outcome(
    value: AdminBillingActionResult,
) -> Result<BackofficeBillingAdminActionOutcome, AppError> {
    Ok(BackofficeBillingAdminActionOutcome {
        object_id: parse_uuid(&value.object_id)?,
        ledger_entry_count: value.ledger_entry_count,
        audit_action: value.audit_action,
        target_type: value.target_type,
        provider: value.provider,
        provider_event_id: value.provider_event_id,
        status: value.status,
    })
}

fn billing_admin_action_kind(value: BackofficeBillingAdminActionKind) -> AdminBillingActionKind {
    match value {
        BackofficeBillingAdminActionKind::CreateCreditNote => {
            AdminBillingActionKind::CreateCreditNote
        }
        BackofficeBillingAdminActionKind::CreateWriteOff => AdminBillingActionKind::CreateWriteOff,
        BackofficeBillingAdminActionKind::CreateRefundIntent => {
            AdminBillingActionKind::CreateRefundIntent
        }
        BackofficeBillingAdminActionKind::CreateManualCompensation => {
            AdminBillingActionKind::CreateManualCompensation
        }
        BackofficeBillingAdminActionKind::ReplayProviderEvent => {
            AdminBillingActionKind::ReplayProviderEvent
        }
        BackofficeBillingAdminActionKind::CreateProviderMigration => {
            AdminBillingActionKind::CreateProviderMigration
        }
        BackofficeBillingAdminActionKind::OverrideGracePeriod => {
            AdminBillingActionKind::OverrideGracePeriod
        }
    }
}
