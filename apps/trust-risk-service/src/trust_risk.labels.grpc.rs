use nvbes_trust_risk::{
    label::LabelAssertion,
    proto::nvbes::trust_risk::v1::{
        self as pb, LabelReceipt as PbLabelReceipt,
        trust_risk_label_service_server::TrustRiskLabelService,
    },
};
use prost::Message;
use tonic::{Request, Response, Status};

use crate::{
    app::TrustRiskState,
    auth,
    labels_db::{LabelPersistenceError, persist_label},
};

#[derive(Clone)]
pub struct LabelService {
    state: TrustRiskState,
}

impl LabelService {
    pub fn new(state: TrustRiskState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl TrustRiskLabelService for LabelService {
    async fn submit_labels(
        &self,
        request: Request<pb::SubmitLabelsRequest>,
    ) -> Result<Response<pb::SubmitLabelsResponse>, Status> {
        if request.get_ref().encoded_len() > 256 * 1024 {
            return Err(Status::resource_exhausted("request exceeds size budget"));
        }
        let metadata = request.metadata().clone();
        let request = request.into_inner();
        if request.labels.is_empty() || request.labels.len() > 128 {
            return Err(Status::invalid_argument("label batch is invalid"));
        }
        let mut receipts = Vec::with_capacity(request.labels.len());
        for wire in request.labels {
            let label = LabelAssertion::try_from(wire)
                .map_err(|_| Status::invalid_argument("label is invalid"))?;
            authorize(&metadata, &self.state, &label)?;
            let receipt = persist_label(
                &self.state.db,
                &label,
                self.state.config.retention.labels_days,
            )
            .await
            .map_err(|error| map_error(&self.state, error))?;
            receipts.push(PbLabelReceipt {
                label_id: receipt.id.to_string(),
                accepted_at: Some(timestamp(receipt.accepted_at)),
                duplicate: receipt.duplicate,
            });
        }
        Ok(Response::new(pb::SubmitLabelsResponse { receipts }))
    }
}

fn authorize(
    metadata: &tonic::metadata::MetadataMap,
    state: &TrustRiskState,
    label: &LabelAssertion,
) -> Result<(), Status> {
    if label.source_class() == pb::LabelSourceClass::Human {
        let actor = label
            .actor()
            .ok_or_else(|| Status::invalid_argument("human actor is required"))?;
        auth::operator(metadata, &state.config, actor, "labels:human")?;
    } else {
        let policy = auth::producer(metadata, &state.config, label.producer())?;
        if !policy.can_label {
            return Err(Status::permission_denied(
                "label submission is not permitted",
            ));
        }
    }
    Ok(())
}

fn timestamp(value: chrono::DateTime<chrono::Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: value.timestamp(),
        nanos: value.timestamp_subsec_nanos() as i32,
    }
}

fn map_error(state: &TrustRiskState, error: LabelPersistenceError) -> Status {
    if matches!(&error, LabelPersistenceError::Database(_)) {
        crate::error_reporting::capture_operation(&state.config, "label.persist", &error);
    }
    match error {
        LabelPersistenceError::Conflict => Status::already_exists("label identifier conflicts"),
        LabelPersistenceError::InvalidCorrection => {
            Status::failed_precondition("correction target is invalid")
        }
        LabelPersistenceError::Database(_) => Status::unavailable("label persistence unavailable"),
    }
}
