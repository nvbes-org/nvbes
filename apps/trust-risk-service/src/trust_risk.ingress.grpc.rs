use nvbes_trust_risk::{
    proto::nvbes::trust_risk::v1::{
        self as pb, SignalReceipt as PbSignalReceipt,
        trust_risk_signal_service_server::TrustRiskSignalService,
    },
    signal::RiskSignal,
};
use prost::Message;
use tonic::{Request, Response, Status};

use crate::{
    app::TrustRiskState,
    auth,
    ingress_db::{PersistSignalError, persist_signal},
};

const MAX_TRANSPORT_BYTES: usize = 256 * 1024;

#[derive(Clone)]
pub struct SignalService {
    state: TrustRiskState,
}

impl SignalService {
    pub fn new(state: TrustRiskState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl TrustRiskSignalService for SignalService {
    async fn submit_signals(
        &self,
        request: Request<pb::SubmitSignalsRequest>,
    ) -> Result<Response<pb::SubmitSignalsResponse>, Status> {
        if request.get_ref().encoded_len() > MAX_TRANSPORT_BYTES {
            return Err(Status::resource_exhausted("request exceeds size budget"));
        }
        let metadata = request.metadata().clone();
        let request = request.into_inner();
        if request.signals.is_empty() || request.signals.len() > 128 {
            return Err(Status::invalid_argument("signal batch is invalid"));
        }
        let mut receipts = Vec::with_capacity(request.signals.len());
        for wire in request.signals {
            let signal = RiskSignal::try_from(wire.clone())
                .map_err(|_| Status::invalid_argument("signal is invalid"))?;
            let policy = auth::producer(&metadata, &self.state.config, signal.producer())?;
            if !policy.permits_signal(signal.kind()) {
                return Err(Status::permission_denied("signal family is not permitted"));
            }
            let receipt = persist_signal(
                &self.state.db,
                &wire,
                &signal,
                self.state.config.retention.signals_days,
            )
            .await
            .map_err(map_persistence)?;
            crate::risk_metrics::signal(
                signal.producer(),
                signal.kind().split('.').next().unwrap_or("unknown"),
                if receipt.duplicate {
                    "duplicate"
                } else {
                    "accepted"
                },
            );
            receipts.push(PbSignalReceipt {
                signal_id: receipt.id.to_string(),
                accepted_at: Some(prost_types::Timestamp {
                    seconds: receipt.accepted_at.timestamp(),
                    nanos: receipt.accepted_at.timestamp_subsec_nanos() as i32,
                }),
                duplicate: receipt.duplicate,
            });
        }
        Ok(Response::new(pb::SubmitSignalsResponse { receipts }))
    }
}

fn map_persistence(error: PersistSignalError) -> Status {
    match error {
        PersistSignalError::Conflict => Status::already_exists("signal identifier conflicts"),
        PersistSignalError::PayloadTooLarge => {
            Status::resource_exhausted("signal exceeds persistence budget")
        }
        PersistSignalError::Database(_) => Status::unavailable("signal persistence unavailable"),
    }
}
