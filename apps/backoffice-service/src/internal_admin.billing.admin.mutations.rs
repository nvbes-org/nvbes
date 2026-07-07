use sqlx::PgPool;
use uuid::Uuid;

use crate::billing_admin_action_log::{audit_action, record_admin_audit};
use crate::billing_admin_types::{
    BackofficeAccess, CreditNoteRequest, GraceOverrideRequest, ManualCompRequest, MutationResult,
    ProviderMigrationRequest, ProviderReplayRequest, ProviderReplayResult, RefundIntentRequest,
};
use crate::billing_admin_validation::{validate_admin_mutation, validate_provider_code};
use crate::billing_grpc_admin::{
    BackofficeBillingAdminActionKind, BackofficeBillingAdminActionOutcome, run_billing_admin_action,
};
use crate::error::AppError;
use crate::grpc_pb::nvbes::billing::v1::AdminBillingActionRequest;

pub(crate) async fn create_credit_note(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    request: CreditNoteRequest,
) -> Result<MutationResult, AppError> {
    validate_admin_mutation(request.amount_minor, &request.reason)?;
    let outcome = run_billing_admin_action(
        billing_grpc_endpoint,
        access.tenant_id,
        workspace_id,
        access.actor_principal_id,
        BackofficeBillingAdminActionKind::CreateCreditNote,
        AdminBillingActionRequest {
            invoice_id: request.invoice_id.to_string(),
            amount_minor: request.amount_minor,
            currency: request.currency,
            reason: request.reason.clone(),
            ..Default::default()
        },
    )
    .await?;
    record_admin_audit(db, access, &request.reason, &outcome).await?;
    mutation_result(outcome)
}

pub(crate) async fn create_write_off(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    request: CreditNoteRequest,
) -> Result<MutationResult, AppError> {
    validate_admin_mutation(request.amount_minor, &request.reason)?;
    let outcome = run_billing_admin_action(
        billing_grpc_endpoint,
        access.tenant_id,
        workspace_id,
        access.actor_principal_id,
        BackofficeBillingAdminActionKind::CreateWriteOff,
        AdminBillingActionRequest {
            invoice_id: request.invoice_id.to_string(),
            amount_minor: request.amount_minor,
            currency: request.currency,
            reason: request.reason.clone(),
            ..Default::default()
        },
    )
    .await?;
    record_admin_audit(db, access, &request.reason, &outcome).await?;
    mutation_result(outcome)
}

pub(crate) async fn create_refund_intent(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    request: RefundIntentRequest,
) -> Result<MutationResult, AppError> {
    validate_admin_mutation(request.amount_minor, &request.reason)?;
    validate_provider_code(&request.provider)?;
    let outcome = run_billing_admin_action(
        billing_grpc_endpoint,
        access.tenant_id,
        workspace_id,
        access.actor_principal_id,
        BackofficeBillingAdminActionKind::CreateRefundIntent,
        AdminBillingActionRequest {
            payment_id: request.payment_id.to_string(),
            amount_minor: request.amount_minor,
            currency: request.currency,
            provider: request.provider,
            reason: request.reason.clone(),
            ..Default::default()
        },
    )
    .await?;
    record_admin_audit(db, access, &request.reason, &outcome).await?;
    mutation_result(outcome)
}

pub(crate) async fn replay_provider_event(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    request: ProviderReplayRequest,
) -> Result<ProviderReplayResult, AppError> {
    validate_admin_mutation(1, &request.reason)?;
    validate_provider_code(&request.provider)?;
    let outcome = run_billing_admin_action(
        billing_grpc_endpoint,
        access.tenant_id,
        workspace_id,
        access.actor_principal_id,
        BackofficeBillingAdminActionKind::ReplayProviderEvent,
        AdminBillingActionRequest {
            provider: request.provider,
            provider_event_id: request.provider_event_id,
            reason: request.reason.clone(),
            amount_minor: 1,
            ..Default::default()
        },
    )
    .await?;
    record_admin_audit(db, access, &request.reason, &outcome).await?;
    Ok(ProviderReplayResult {
        object_id: outcome.object_id,
        provider: outcome.provider,
        provider_event_id: outcome.provider_event_id,
        status: outcome.status,
        audit_action: audit_action(&outcome.audit_action)?,
    })
}

pub(crate) async fn create_provider_migration(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    request: ProviderMigrationRequest,
) -> Result<MutationResult, AppError> {
    validate_admin_mutation(1, &request.reason)?;
    validate_provider_code(&request.from_provider)?;
    validate_provider_code(&request.to_provider)?;
    if request.from_provider == request.to_provider {
        return Err(AppError::bad_request(
            "invalid_provider_migration",
            "Provider migration requires distinct source and target providers.",
        ));
    }
    let outcome = run_billing_admin_action(
        billing_grpc_endpoint,
        access.tenant_id,
        workspace_id,
        access.actor_principal_id,
        BackofficeBillingAdminActionKind::CreateProviderMigration,
        AdminBillingActionRequest {
            from_provider: request.from_provider,
            to_provider: request.to_provider,
            reason: request.reason.clone(),
            amount_minor: 1,
            ..Default::default()
        },
    )
    .await?;
    record_admin_audit(db, access, &request.reason, &outcome).await?;
    mutation_result(outcome)
}

pub(crate) async fn override_grace_period(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    request: GraceOverrideRequest,
) -> Result<MutationResult, AppError> {
    validate_admin_mutation(request.grace_days, &request.reason)?;
    let outcome = run_billing_admin_action(
        billing_grpc_endpoint,
        access.tenant_id,
        workspace_id,
        access.actor_principal_id,
        BackofficeBillingAdminActionKind::OverrideGracePeriod,
        AdminBillingActionRequest {
            subscription_id: request
                .subscription_id
                .map_or_else(String::new, |id| id.to_string()),
            grace_days: request.grace_days,
            reason: request.reason.clone(),
            amount_minor: request.grace_days,
            ..Default::default()
        },
    )
    .await?;
    record_admin_audit(db, access, &request.reason, &outcome).await?;
    mutation_result(outcome)
}

pub(crate) async fn create_manual_compensation(
    db: &PgPool,
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    request: ManualCompRequest,
) -> Result<MutationResult, AppError> {
    validate_admin_mutation(request.amount_minor, &request.reason)?;
    validate_manual_comp_direction(&request.direction)?;
    let outcome = run_billing_admin_action(
        billing_grpc_endpoint,
        access.tenant_id,
        workspace_id,
        access.actor_principal_id,
        BackofficeBillingAdminActionKind::CreateManualCompensation,
        AdminBillingActionRequest {
            amount_minor: request.amount_minor,
            currency: request.currency,
            direction: request.direction,
            reason: request.reason.clone(),
            ..Default::default()
        },
    )
    .await?;
    record_admin_audit(db, access, &request.reason, &outcome).await?;
    mutation_result(outcome)
}

fn mutation_result(
    outcome: BackofficeBillingAdminActionOutcome,
) -> Result<MutationResult, AppError> {
    Ok(MutationResult {
        object_id: outcome.object_id,
        ledger_entry_count: outcome.ledger_entry_count,
        audit_action: audit_action(&outcome.audit_action)?,
    })
}

fn validate_manual_comp_direction(direction: &str) -> Result<(), AppError> {
    match direction {
        "credit" | "debit" => Ok(()),
        _ => Err(AppError::bad_request(
            "invalid_adjustment_direction",
            "Manual compensation direction must be credit or debit.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_comp_direction_validation_is_directional() {
        assert!(validate_manual_comp_direction("credit").is_ok());
        assert!(validate_manual_comp_direction("debit").is_ok());
        assert!(validate_manual_comp_direction("invalid").is_err());
    }

    #[test]
    fn provider_code_validation_accepts_supported_psps_only() {
        assert!(validate_provider_code("stripe").is_ok());
        assert!(validate_provider_code("mollie").is_ok());
        assert!(validate_provider_code("paypal").is_err());
    }
}
