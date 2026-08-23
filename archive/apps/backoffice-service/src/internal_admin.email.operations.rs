use std::sync::Arc;

use nvbes_email::{
    EmailOperationsClient, EmailOperationsError,
    proto::nvbes::{
        email::v1::{
            ApplyEmailSuppressionRequest, EmailCallerContext, EmailOperationsSnapshot,
            EmailOperatorActionReceipt, EmailOperatorContext, GetEmailOperationsSnapshotRequest,
            ReleaseEmailSuppressionRequest, ReplayEmailRequest, ReviewEmailSuppressionRequest,
        },
        platform::v1::{RequestContext, TenantContext},
    },
};
use uuid::Uuid;

use crate::{billing_admin_types::BackofficeAccess, error::AppError};

const CALLER: &str = "backoffice-service";

#[tonic::async_trait]
pub(crate) trait BackofficeEmailOperations: Send + Sync {
    async fn snapshot(&self, actor: Uuid) -> Result<EmailOperationsSnapshot, AppError>;
    async fn replay_email(
        &self,
        access: BackofficeAccess,
        workspace_id: Uuid,
        message_id: Uuid,
        reason: &str,
    ) -> Result<EmailOperatorActionReceipt, AppError>;
    async fn apply_suppression(
        &self,
        access: BackofficeAccess,
        workspace_id: Uuid,
        email: &str,
        reason: &str,
    ) -> Result<EmailOperatorActionReceipt, AppError>;
    async fn release_suppression(
        &self,
        access: BackofficeAccess,
        workspace_id: Uuid,
        email: &str,
        reason: &str,
    ) -> Result<EmailOperatorActionReceipt, AppError>;
    async fn review_suppression(
        &self,
        access: BackofficeAccess,
        workspace_id: Uuid,
        email: &str,
        reason: &str,
    ) -> Result<EmailOperatorActionReceipt, AppError>;
}

pub(crate) fn from_environment(
    environment: &str,
) -> Result<Arc<dyn BackofficeEmailOperations>, String> {
    EmailOperationsClient::from_environment(environment)
        .map(|client| {
            Arc::new(GrpcEmailOperations { client }) as Arc<dyn BackofficeEmailOperations>
        })
        .map_err(|error| error.to_string())
}

struct GrpcEmailOperations {
    client: EmailOperationsClient,
}

#[tonic::async_trait]
impl BackofficeEmailOperations for GrpcEmailOperations {
    async fn snapshot(&self, actor: Uuid) -> Result<EmailOperationsSnapshot, AppError> {
        self.client
            .snapshot(GetEmailOperationsSnapshotRequest {
                caller: Some(caller_context(actor, None, None)),
            })
            .await
            .map_err(map_error)
    }

    async fn replay_email(
        &self,
        access: BackofficeAccess,
        workspace_id: Uuid,
        message_id: Uuid,
        reason: &str,
    ) -> Result<EmailOperatorActionReceipt, AppError> {
        self.client
            .replay_email(ReplayEmailRequest {
                operator: Some(operator_context(access, workspace_id, reason)),
                message_id: message_id.to_string(),
            })
            .await
            .map_err(map_error)
    }

    async fn apply_suppression(
        &self,
        access: BackofficeAccess,
        workspace_id: Uuid,
        email: &str,
        reason: &str,
    ) -> Result<EmailOperatorActionReceipt, AppError> {
        self.client
            .apply_suppression(ApplyEmailSuppressionRequest {
                operator: Some(operator_context(access, workspace_id, reason)),
                email: email.to_string(),
                scope: "all".to_string(),
            })
            .await
            .map_err(map_error)
    }

    async fn release_suppression(
        &self,
        access: BackofficeAccess,
        workspace_id: Uuid,
        email: &str,
        reason: &str,
    ) -> Result<EmailOperatorActionReceipt, AppError> {
        self.client
            .release_suppression(ReleaseEmailSuppressionRequest {
                operator: Some(operator_context(access, workspace_id, reason)),
                email: email.to_string(),
            })
            .await
            .map_err(map_error)
    }

    async fn review_suppression(
        &self,
        access: BackofficeAccess,
        workspace_id: Uuid,
        email: &str,
        reason: &str,
    ) -> Result<EmailOperatorActionReceipt, AppError> {
        self.client
            .review_suppression(ReviewEmailSuppressionRequest {
                operator: Some(operator_context(access, workspace_id, reason)),
                email: email.to_string(),
            })
            .await
            .map_err(map_error)
    }
}

fn operator_context(
    access: BackofficeAccess,
    workspace_id: Uuid,
    reason: &str,
) -> EmailOperatorContext {
    EmailOperatorContext {
        caller: Some(caller_context(
            access.actor_principal_id,
            Some(access.tenant_id),
            Some(workspace_id),
        )),
        actor: access.actor_principal_id.to_string(),
        reason: reason.to_string(),
    }
}

fn caller_context(
    actor: Uuid,
    tenant_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
) -> EmailCallerContext {
    EmailCallerContext {
        request_context: Some(RequestContext {
            request_id: Uuid::new_v4().to_string(),
            correlation_id: Uuid::new_v4().to_string(),
            actor_principal_id: actor.to_string(),
            tenant: tenant_id.map(|tenant_id| TenantContext {
                tenant_id: tenant_id.to_string(),
                workspace_id: workspace_id.map(|id| id.to_string()).unwrap_or_default(),
                region_id: CALLER.to_string(),
                data_residency: "internal".to_string(),
            }),
        }),
        caller: CALLER.to_string(),
    }
}

fn map_error(error: EmailOperationsError) -> AppError {
    match error {
        EmailOperationsError::Invalid => {
            AppError::bad_request("email_operation_invalid", error.to_string())
        }
        EmailOperationsError::NotFound => {
            AppError::not_found("email_operation_not_found", error.to_string())
        }
        EmailOperationsError::Conflict => {
            AppError::conflict("email_operation_conflict", error.to_string())
        }
        EmailOperationsError::Unauthorized => {
            AppError::forbidden("email_operation_forbidden", error.to_string())
        }
        EmailOperationsError::Unavailable
        | EmailOperationsError::Protocol
        | EmailOperationsError::Configuration(_) => {
            AppError::internal("email_operation_unavailable", error.to_string())
        }
    }
}

#[cfg(test)]
pub(crate) fn test_gateway() -> Arc<dyn BackofficeEmailOperations> {
    Arc::new(TestEmailOperations)
}

#[cfg(test)]
struct TestEmailOperations;

#[cfg(test)]
#[tonic::async_trait]
impl BackofficeEmailOperations for TestEmailOperations {
    async fn snapshot(&self, _actor: Uuid) -> Result<EmailOperationsSnapshot, AppError> {
        Ok(EmailOperationsSnapshot::default())
    }

    async fn replay_email(
        &self,
        _access: BackofficeAccess,
        _workspace_id: Uuid,
        _message_id: Uuid,
        _reason: &str,
    ) -> Result<EmailOperatorActionReceipt, AppError> {
        Ok(receipt("queued"))
    }

    async fn apply_suppression(
        &self,
        _access: BackofficeAccess,
        _workspace_id: Uuid,
        _email: &str,
        _reason: &str,
    ) -> Result<EmailOperatorActionReceipt, AppError> {
        Ok(receipt("applied"))
    }

    async fn release_suppression(
        &self,
        _access: BackofficeAccess,
        _workspace_id: Uuid,
        _email: &str,
        _reason: &str,
    ) -> Result<EmailOperatorActionReceipt, AppError> {
        Ok(receipt("released"))
    }

    async fn review_suppression(
        &self,
        _access: BackofficeAccess,
        _workspace_id: Uuid,
        _email: &str,
        _reason: &str,
    ) -> Result<EmailOperatorActionReceipt, AppError> {
        Ok(receipt("reviewed"))
    }
}

#[cfg(test)]
fn receipt(status: &str) -> EmailOperatorActionReceipt {
    EmailOperatorActionReceipt {
        action_id: Uuid::new_v4().to_string(),
        status: status.to_string(),
    }
}
