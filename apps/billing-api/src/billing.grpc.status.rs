use chrono::{DateTime, Utc};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::pb::nvbes::platform::v1::RequestContext;

pub fn validate_context(
    context: Option<&RequestContext>,
    workspace_id: Option<Uuid>,
) -> Result<&RequestContext, Status> {
    let context = context.ok_or_else(|| {
        Status::invalid_argument("request context is required for Billing gRPC calls")
    })?;
    if context.request_id.trim().is_empty() || context.actor_principal_id.trim().is_empty() {
        return Err(Status::invalid_argument(
            "request_id and actor_principal_id are required in request context",
        ));
    }
    if let Some(workspace_id) = workspace_id {
        let tenant = context.tenant.as_ref().ok_or_else(|| {
            Status::invalid_argument("tenant context is required for workspace Billing calls")
        })?;
        if tenant.workspace_id != workspace_id.to_string() {
            return Err(Status::permission_denied(
                "request context workspace does not match the Billing workspace",
            ));
        }
    }
    Ok(context)
}

pub fn workspace_id(value: &str) -> Result<Uuid, Status> {
    parse_uuid(value, "workspace_id")
}

pub fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, Status> {
    Uuid::parse_str(value)
        .map_err(|_| Status::invalid_argument(format!("{field} must be a valid UUID")))
}

pub fn optional_uuid(value: &str, field: &'static str) -> Result<Option<Uuid>, Status> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_uuid(value, field).map(Some)
    }
}

pub fn parse_datetime(value: &str, field: &'static str) -> Result<DateTime<Utc>, Status> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| Status::invalid_argument(format!("{field} must be an RFC3339 timestamp")))
}

pub fn empty_to_none(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

pub fn sql_status(error: sqlx::Error) -> Status {
    match error {
        sqlx::Error::RowNotFound => Status::not_found("billing workspace was not found"),
        error => Status::internal(format!("billing database error: {error}")),
    }
}

pub fn checkout_status(
    error: nvbes_billing::checkout_sessions::BillingCheckoutSessionError,
) -> Status {
    use nvbes_billing::checkout_sessions::BillingCheckoutSessionError;

    match error {
        BillingCheckoutSessionError::WorkspaceNotFound => {
            Status::not_found("billing workspace was not found")
        }
        BillingCheckoutSessionError::InvalidRedirect { field, .. } => {
            Status::invalid_argument(format!("{field} is not an allowed redirect URL"))
        }
        BillingCheckoutSessionError::BillingLocked
        | BillingCheckoutSessionError::ManualReviewHold
        | BillingCheckoutSessionError::FraudBlocked
        | BillingCheckoutSessionError::RoutingBlocked(_) => {
            Status::failed_precondition(error.to_string())
        }
        BillingCheckoutSessionError::FraudPolicy(_)
        | BillingCheckoutSessionError::RoutingRule(_)
        | BillingCheckoutSessionError::CheckoutProvider(_)
        | BillingCheckoutSessionError::Database(_) => Status::internal(error.to_string()),
    }
}

pub fn usage_status(error: nvbes_billing::usage::BillingUsageIngestError) -> Status {
    match error {
        nvbes_billing::usage::BillingUsageIngestError::Validation { code, message } => {
            Status::invalid_argument(format!("{code}: {message}"))
        }
        nvbes_billing::usage::BillingUsageIngestError::Database(_)
        | nvbes_billing::usage::BillingUsageIngestError::Serialization(_) => {
            Status::internal(error.to_string())
        }
    }
}

pub fn reconciliation_status(
    error: nvbes_billing::reconciliation_db::BillingReconciliationError,
) -> Status {
    Status::internal(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::validate_context;
    use crate::grpc::pb::nvbes::platform::v1::{RequestContext, TenantContext};
    use uuid::Uuid;

    #[test]
    fn context_workspace_must_match_request_workspace() {
        let workspace_id = Uuid::new_v4();
        let context = RequestContext {
            request_id: "req_123".to_string(),
            correlation_id: "corr_123".to_string(),
            actor_principal_id: Uuid::new_v4().to_string(),
            tenant: Some(TenantContext {
                tenant_id: Uuid::new_v4().to_string(),
                workspace_id: workspace_id.to_string(),
                region_id: "eu".to_string(),
                data_residency: "eu".to_string(),
            }),
        };

        assert!(validate_context(Some(&context), Some(workspace_id)).is_ok());
        assert!(
            validate_context(Some(&context), Some(Uuid::new_v4()))
                .expect_err("mismatched workspace should be rejected")
                .message()
                .contains("workspace")
        );
    }
}
