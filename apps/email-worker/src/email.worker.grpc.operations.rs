use tonic::{Request, Response, Status};
use uuid::Uuid;

use nvbes_email::proto::nvbes::email::v1::{
    ApplyEmailSuppressionRequest, EmailCallerContext, EmailOperationsSnapshot,
    EmailOperatorActionReceipt, EmailOperatorContext, EmailPrivacyActivity,
    GetEmailOperationsSnapshotRequest, GetEmailPrivacyActivityRequest,
    ReleaseEmailSuppressionRequest, ReplayEmailRequest, ReviewEmailSuppressionRequest,
    email_operations_service_server::EmailOperationsService,
};

use crate::{
    auth,
    operations_actions::{self, ActionError, OperatorAction},
    operations_privacy, operations_snapshot,
    state::EmailWorkerState,
};

const PLATFORM_OPERATIONS_CALLER: &str = "platform-operations-service";
const PRIVACY_CALLER: &str = "identity-service";

#[derive(Clone)]
pub struct EmailOperationsGrpcService {
    state: EmailWorkerState,
}

impl EmailOperationsGrpcService {
    pub fn new(state: EmailWorkerState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl EmailOperationsService for EmailOperationsGrpcService {
    async fn get_operations_snapshot(
        &self,
        request: Request<GetEmailOperationsSnapshotRequest>,
    ) -> Result<Response<EmailOperationsSnapshot>, Status> {
        authenticate_caller(
            &request,
            &self.state,
            request.get_ref().caller.as_ref(),
            PLATFORM_OPERATIONS_CALLER,
        )?;
        operations_snapshot::load(&self.state.db, &self.state.crypto)
            .await
            .map(Response::new)
            .map_err(internal)
    }

    async fn replay_email(
        &self,
        request: Request<ReplayEmailRequest>,
    ) -> Result<Response<EmailOperatorActionReceipt>, Status> {
        let operator =
            authenticated_operator(&request, &self.state, request.get_ref().operator.as_ref())?;
        let message_id = Uuid::parse_str(&request.get_ref().message_id)
            .map_err(|_| Status::invalid_argument("message_id must be a UUID"))?;
        operations_actions::replay_email(&self.state.db, message_id, operator)
            .await
            .map(action_receipt)
            .map(Response::new)
            .map_err(action_error)
    }

    async fn apply_suppression(
        &self,
        request: Request<ApplyEmailSuppressionRequest>,
    ) -> Result<Response<EmailOperatorActionReceipt>, Status> {
        let operator =
            authenticated_operator(&request, &self.state, request.get_ref().operator.as_ref())?;
        operations_actions::apply_suppression(
            &self.state.db,
            &self.state.crypto,
            &request.get_ref().email,
            &request.get_ref().scope,
            operator,
        )
        .await
        .map(action_receipt)
        .map(Response::new)
        .map_err(action_error)
    }

    async fn release_suppression(
        &self,
        request: Request<ReleaseEmailSuppressionRequest>,
    ) -> Result<Response<EmailOperatorActionReceipt>, Status> {
        let operator =
            authenticated_operator(&request, &self.state, request.get_ref().operator.as_ref())?;
        operations_actions::release_suppression(
            &self.state.db,
            &self.state.crypto,
            &request.get_ref().email,
            operator,
        )
        .await
        .map(action_receipt)
        .map(Response::new)
        .map_err(action_error)
    }

    async fn review_suppression(
        &self,
        request: Request<ReviewEmailSuppressionRequest>,
    ) -> Result<Response<EmailOperatorActionReceipt>, Status> {
        let operator =
            authenticated_operator(&request, &self.state, request.get_ref().operator.as_ref())?;
        operations_actions::review_suppression(
            &self.state.db,
            &self.state.crypto,
            &request.get_ref().email,
            operator,
        )
        .await
        .map(action_receipt)
        .map(Response::new)
        .map_err(action_error)
    }

    async fn get_privacy_activity(
        &self,
        request: Request<GetEmailPrivacyActivityRequest>,
    ) -> Result<Response<EmailPrivacyActivity>, Status> {
        authenticate_caller(
            &request,
            &self.state,
            request.get_ref().caller.as_ref(),
            PRIVACY_CALLER,
        )?;
        request
            .get_ref()
            .email
            .parse::<lettre::Address>()
            .map_err(|_| Status::invalid_argument("email is invalid"))?;
        operations_privacy::load(&self.state.db, &self.state.crypto, &request.get_ref().email)
            .await
            .map(Response::new)
            .map_err(internal)
    }
}

fn authenticated_operator<'a, T>(
    request: &Request<T>,
    state: &EmailWorkerState,
    operator: Option<&'a EmailOperatorContext>,
) -> Result<OperatorAction<'a>, Status> {
    let operator = operator.ok_or_else(|| Status::invalid_argument("operator is required"))?;
    authenticate_caller(
        request,
        state,
        operator.caller.as_ref(),
        PLATFORM_OPERATIONS_CALLER,
    )?;
    if Uuid::parse_str(&operator.actor).is_err()
        || operator.reason.trim().len() < 12
        || operator.reason.len() > 500
        || operator.reason.contains(['\r', '\n'])
    {
        return Err(Status::invalid_argument(
            "operator audit context is invalid",
        ));
    }
    Ok(OperatorAction {
        actor: &operator.actor,
        reason: &operator.reason,
    })
}

fn authenticate_caller<T>(
    request: &Request<T>,
    state: &EmailWorkerState,
    caller: Option<&EmailCallerContext>,
    expected: &str,
) -> Result<(), Status> {
    let caller = caller.ok_or_else(|| Status::invalid_argument("caller is required"))?;
    if caller.caller != expected || caller.request_context.is_none() {
        return Err(Status::permission_denied(
            "email operation is not permitted",
        ));
    }
    auth::authenticate_producer(request, &state.config.producer_tokens, &caller.caller)
}

fn action_receipt(value: operations_actions::ActionReceipt) -> EmailOperatorActionReceipt {
    EmailOperatorActionReceipt {
        action_id: value.id.to_string(),
        status: value.status.to_string(),
    }
}

fn action_error(error: ActionError) -> Status {
    match error {
        ActionError::Invalid => Status::invalid_argument("email operation is invalid"),
        ActionError::NotFound => Status::not_found("email operation target was not found"),
        ActionError::Precondition => {
            Status::failed_precondition("email cannot be replayed in its current state")
        }
        ActionError::Database(_) | ActionError::Encryption(_) => {
            tracing::error!(error = ?error, "email operator action failed");
            Status::unavailable("email operator action failed")
        }
    }
}

fn internal(error: impl std::fmt::Debug) -> Status {
    tracing::error!(error = ?error, "email operations query failed");
    Status::unavailable("email operations query failed")
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "email.worker.grpc.operations.tests.rs"]
mod tests;
