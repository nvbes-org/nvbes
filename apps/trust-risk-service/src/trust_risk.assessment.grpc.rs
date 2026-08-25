use nvbes_trust_risk::{
    assessment::Assessment,
    proto::nvbes::trust_risk::v1::{
        self as pb, trust_risk_assessment_service_server::TrustRiskAssessmentService,
    },
    types::{Recommendation, RiskBand},
};
use prost::Message;
use tonic::{Request, Response, Status};

use crate::{
    app::TrustRiskState,
    assessment_db::{StoredEvaluation, assess},
    assessment_error::AssessmentPersistenceError,
    auth,
};

#[derive(Clone)]
pub struct AssessmentService {
    state: TrustRiskState,
}

impl AssessmentService {
    pub fn new(state: TrustRiskState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl TrustRiskAssessmentService for AssessmentService {
    async fn assess_risk(
        &self,
        request: Request<pb::AssessRiskRequest>,
    ) -> Result<Response<pb::RiskEvaluation>, Status> {
        let started = std::time::Instant::now();
        if request.get_ref().encoded_len() > 256 * 1024 {
            return Err(Status::resource_exhausted("request exceeds size budget"));
        }
        let metadata = request.metadata().clone();
        let wire = request.into_inner();
        let assessment = Assessment::try_from(wire.clone())
            .map_err(|_| Status::invalid_argument("assessment is invalid"))?;
        let policy = auth::producer(&metadata, &self.state.config, assessment.producer())?;
        if !policy.can_assess {
            return Err(Status::permission_denied("assessment is not permitted"));
        }
        let stored = assess(
            &self.state.db,
            &wire,
            &assessment,
            self.state.config.retention.evaluations_days,
            self.state.config.retention.signals_days,
            self.state.config.retention.reviews_days,
        )
        .await
        .map_err(|error| map_error(&self.state, error))?;
        crate::risk_metrics::assessment(
            recommendation_name(stored.recommendation),
            "ok",
            started.elapsed(),
        );
        Ok(Response::new(to_proto(stored)))
    }
}

fn recommendation_name(value: Recommendation) -> &'static str {
    match value {
        Recommendation::Allow => "allow",
        Recommendation::Challenge => "challenge",
        Recommendation::Review => "review",
        Recommendation::Deny => "deny",
    }
}

fn to_proto(value: StoredEvaluation) -> pb::RiskEvaluation {
    pb::RiskEvaluation {
        evaluation_id: value.id.to_string(),
        score: u32::from(value.score),
        band: match value.band {
            RiskBand::Low => pb::RiskBand::Low,
            RiskBand::Elevated => pb::RiskBand::Elevated,
            RiskBand::High => pb::RiskBand::High,
            RiskBand::Critical => pb::RiskBand::Critical,
        }
        .into(),
        recommendation: match value.recommendation {
            Recommendation::Allow => pb::RiskRecommendation::Allow,
            Recommendation::Challenge => pb::RiskRecommendation::Challenge,
            Recommendation::Review => pb::RiskRecommendation::Review,
            Recommendation::Deny => pb::RiskRecommendation::Deny,
        }
        .into(),
        reasons: value
            .reasons
            .into_iter()
            .map(|code| pb::RiskReason {
                code,
                parameters: Default::default(),
            })
            .collect(),
        feature_version: value.feature_version,
        rule_set_version: value.rule_set_version,
        evaluated_at: Some(timestamp(value.evaluated_at)),
        expires_at: Some(timestamp(value.expires_at)),
        duplicate: value.duplicate,
    }
}

fn timestamp(value: chrono::DateTime<chrono::Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: value.timestamp(),
        nanos: value.timestamp_subsec_nanos() as i32,
    }
}

fn map_error(state: &TrustRiskState, error: AssessmentPersistenceError) -> Status {
    if matches!(
        &error,
        AssessmentPersistenceError::Database(_)
            | AssessmentPersistenceError::Signal(crate::ingress_db::PersistSignalError::Database(
                _
            ))
            | AssessmentPersistenceError::CorruptFeatureState
            | AssessmentPersistenceError::CorruptLedger
    ) {
        crate::error_reporting::capture_operation(&state.config, "assessment.persist", &error);
    }
    match error {
        AssessmentPersistenceError::Conflict => Status::already_exists("assessment key conflicts"),
        AssessmentPersistenceError::NoActiveRules | AssessmentPersistenceError::InvalidRules => {
            Status::failed_precondition("assessment configuration unavailable")
        }
        AssessmentPersistenceError::Signal(crate::ingress_db::PersistSignalError::Conflict) => {
            Status::already_exists("signal identifier conflicts")
        }
        AssessmentPersistenceError::Signal(
            crate::ingress_db::PersistSignalError::PayloadTooLarge,
        ) => Status::resource_exhausted("evidence exceeds size budget"),
        _ => Status::unavailable("assessment unavailable"),
    }
}
