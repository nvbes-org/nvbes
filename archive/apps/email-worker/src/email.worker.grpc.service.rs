use chrono::Utc;
use prost::Message;
use tonic::{Request, Response, Status};

use nvbes_email::{
    EmailCommand,
    proto::nvbes::email::v1::{
        EmailReceipt as ProtoEmailReceipt, SubmitEmailRequest,
        email_delivery_service_server::{EmailDeliveryService, EmailDeliveryServiceServer},
    },
};

use crate::{auth, database, email_metrics, error_reporting, state::EmailWorkerState};

pub const MAX_GRPC_DECODE_BYTES: usize = 256 * 1024;
pub const MAX_PERSISTED_COMMAND_BYTES: usize = 192 * 1024;

#[derive(Clone)]
pub struct EmailDeliveryGrpcService {
    state: EmailWorkerState,
}

impl EmailDeliveryGrpcService {
    pub fn new(state: EmailWorkerState) -> Self {
        Self { state }
    }
}

pub fn delivery_server(
    service: EmailDeliveryGrpcService,
) -> EmailDeliveryServiceServer<EmailDeliveryGrpcService> {
    EmailDeliveryServiceServer::new(service).max_decoding_message_size(MAX_GRPC_DECODE_BYTES)
}

fn validate_submission_size(request: &SubmitEmailRequest) -> Result<(), Status> {
    if request.encoded_len() > MAX_PERSISTED_COMMAND_BYTES {
        return Err(Status::invalid_argument("email command exceeds size limit"));
    }
    Ok(())
}

#[tonic::async_trait]
impl EmailDeliveryService for EmailDeliveryGrpcService {
    #[tracing::instrument(
        name = "email.submit",
        skip_all,
        fields(
            rpc.system = "grpc",
            rpc.service = "nvbes.email.v1.EmailDeliveryService",
            rpc.method = "SubmitEmail",
            email.producer = tracing::field::Empty,
            email.business_type = tracing::field::Empty,
            email.template_version = tracing::field::Empty,
            otel.status_code = tracing::field::Empty,
        )
    )]
    async fn submit_email(
        &self,
        request: Request<SubmitEmailRequest>,
    ) -> Result<Response<ProtoEmailReceipt>, Status> {
        auth::authenticate_producer(
            &request,
            &self.state.config.producer_tokens,
            &request.get_ref().producer,
        )?;
        validate_submission_size(request.get_ref())?;
        let wire = request.into_inner();
        let command = EmailCommand::try_from(wire.clone())
            .map_err(|_| Status::invalid_argument("invalid email command"))?;
        command
            .validate(Utc::now())
            .map_err(|_| Status::invalid_argument("invalid email command"))?;
        let accepted = database::accept_command(
            &self.state.db,
            &self.state.crypto,
            &self.state.config.message_id_domain,
            &wire,
            &command,
        )
        .await
        .map_err(|error| map_accept_error(error, &self.state))?;
        if accepted.enqueue_required {
            self.state
                .dispatch_queue
                .enqueue(accepted.id)
                .await
                .map_err(|error| {
                    tracing::error!(
                        message_id = %accepted.id,
                        error = ?error,
                        "email command persisted but dispatch enqueue failed"
                    );
                    Status::unavailable("email command dispatch could not be scheduled")
                })?;
        }
        let receipt = accepted.receipt;
        let (template_name, template_version) = command.template.name_and_version();
        let span = tracing::Span::current();
        span.record("email.producer", command.producer.as_str());
        span.record("email.business_type", template_name);
        span.record("email.template_version", template_version);
        email_metrics::accepted(
            &command.producer,
            template_name,
            template_version,
            receipt.duplicate,
        );
        Ok(Response::new(receipt.into_proto()))
    }
}

fn map_accept_error(error: database::AcceptCommandError, state: &EmailWorkerState) -> Status {
    match error {
        database::AcceptCommandError::Invalid | database::AcceptCommandError::Validation(_) => {
            Status::invalid_argument("invalid email command")
        }
        database::AcceptCommandError::Conflict => {
            Status::already_exists("email idempotency key conflict")
        }
        database::AcceptCommandError::Database(_) | database::AcceptCommandError::Encryption(_) => {
            tracing::Span::current().record("otel.status_code", "ERROR");
            error_reporting::capture_operation(&state.config, "grpc_accept_command", &error);
            tracing::error!(error = ?error, "email command could not be persisted");
            Status::unavailable("email command could not be durably accepted")
        }
    }
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "email.worker.grpc.service.tests.rs"]
mod tests;

#[cfg(all(test, feature = "database-tests"))]
#[path = "email.worker.grpc.cdc.tests.rs"]
mod cdc_tests;

#[cfg(test)]
#[path = "email.worker.grpc.service.limits.tests.rs"]
mod limits_tests;
