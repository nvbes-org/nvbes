use chrono::Utc;
use tonic::{Request, Response, Status};

use nvbes_email::{
    EmailCommand,
    proto::nvbes::email::v1::{
        EmailReceipt as ProtoEmailReceipt, SubmitEmailRequest,
        email_delivery_service_server::EmailDeliveryService,
    },
};

use crate::{auth, database, email_metrics, state::EmailWorkerState};

#[derive(Clone)]
pub struct EmailDeliveryGrpcService {
    state: EmailWorkerState,
}

impl EmailDeliveryGrpcService {
    pub fn new(state: EmailWorkerState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl EmailDeliveryService for EmailDeliveryGrpcService {
    async fn submit_email(
        &self,
        request: Request<SubmitEmailRequest>,
    ) -> Result<Response<ProtoEmailReceipt>, Status> {
        auth::authenticate_producer(
            &request,
            &self.state.config.producer_tokens,
            &request.get_ref().producer,
        )?;
        let wire = request.into_inner();
        let command = EmailCommand::try_from(wire.clone())
            .map_err(|_| Status::invalid_argument("invalid email command"))?;
        command
            .validate(Utc::now())
            .map_err(|_| Status::invalid_argument("invalid email command"))?;
        let receipt = database::accept_command(
            &self.state.db,
            &self.state.crypto,
            &self.state.config.message_id_domain,
            &wire,
            &command,
        )
        .await
        .map_err(map_accept_error)?;
        let (template_name, template_version) = command.template.name_and_version();
        email_metrics::accepted(
            &command.producer,
            template_name,
            template_version,
            receipt.duplicate,
        );
        Ok(Response::new(receipt.into_proto()))
    }
}

fn map_accept_error(error: database::AcceptCommandError) -> Status {
    match error {
        database::AcceptCommandError::Invalid | database::AcceptCommandError::Validation(_) => {
            Status::invalid_argument("invalid email command")
        }
        database::AcceptCommandError::Conflict => {
            Status::already_exists("email idempotency key conflict")
        }
        database::AcceptCommandError::Database(_) | database::AcceptCommandError::Encryption(_) => {
            tracing::error!(error = ?error, "email command could not be persisted");
            Status::unavailable("email command could not be durably accepted")
        }
    }
}
