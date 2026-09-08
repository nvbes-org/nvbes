use nvbes_trust_risk::{
    label::LabelAssertion,
    proto::nvbes::trust_risk::v1::{
        self as pb, trust_risk_operations_service_server::TrustRiskOperationsService,
    },
};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::{
    app::TrustRiskState,
    auth, labels_db,
    operations_types::{
        EvaluationParams, evaluation, label_kind_name, map_label, map_review, map_rule,
        review_case, review_case_row, review_state, review_state_name, rule_receipt, unavailable,
        uuid,
    },
    review_db, rules_db,
};

#[derive(Clone)]
pub struct OperationsService {
    state: TrustRiskState,
}

impl OperationsService {
    pub fn new(state: TrustRiskState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl TrustRiskOperationsService for OperationsService {
    async fn get_evaluation(
        &self,
        request: Request<pb::GetEvaluationRequest>,
    ) -> Result<Response<pb::RiskEvaluationDetail>, Status> {
        let (metadata, request) = parts(request);
        authorize(
            &metadata,
            &self.state,
            request.operator.as_ref(),
            "evaluation:read",
        )?;
        let id = uuid(&request.evaluation_id)?;
        let row = sqlx::query_as::<_, (String, String, i16, String, String, String, String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>(
            "SELECT producer, operation_class, score, band, recommendation, feature_version, rule_set_version, evaluated_at, expires_at FROM trust_risk_evaluations WHERE id = $1",
        ).bind(id).fetch_optional(&self.state.db).await.map_err(unavailable)?.ok_or_else(|| Status::not_found("evaluation not found"))?;
        let features = sqlx::query_as::<_, (String, f64)>(
            "SELECT name, value FROM trust_risk_evaluation_features WHERE evaluation_id = $1 ORDER BY name",
        ).bind(id).fetch_all(&self.state.db).await.map_err(unavailable)?;
        let reasons = sqlx::query_scalar::<_, String>(
            "SELECT code FROM trust_risk_evaluation_reasons WHERE evaluation_id = $1 ORDER BY ordinal",
        ).bind(id).fetch_all(&self.state.db).await.map_err(unavailable)?;
        let canonical_label = sqlx::query_scalar::<_, i16>(
            "SELECT label.kind FROM trust_risk_canonical_labels AS canonical JOIN trust_risk_labels AS label ON label.id = canonical.label_id WHERE canonical.evaluation_id = $1",
        ).bind(id).fetch_optional(&self.state.db).await.map_err(unavailable)?
            .and_then(label_kind_name).map(str::to_string);
        Ok(Response::new(pb::RiskEvaluationDetail {
            evaluation: Some(evaluation(EvaluationParams {
                id,
                score: row.2,
                band: &row.3,
                recommendation: &row.4,
                reasons,
                feature_version: &row.5,
                rule_version: &row.6,
                evaluated_at: row.7,
                expires_at: row.8,
            })),
            producer: row.0,
            operation_class: row.1,
            features: features
                .into_iter()
                .map(|(name, value)| pb::FeatureValue { name, value })
                .collect(),
            canonical_label,
        }))
    }

    async fn list_review_cases(
        &self,
        request: Request<pb::ListReviewCasesRequest>,
    ) -> Result<Response<pb::ListReviewCasesResponse>, Status> {
        let (metadata, request) = parts(request);
        authorize(
            &metadata,
            &self.state,
            request.operator.as_ref(),
            "evaluation:read",
        )?;
        let state = match pb::ReviewCaseState::try_from(request.state)
            .map_err(|_| Status::invalid_argument("state is invalid"))?
        {
            pb::ReviewCaseState::Unspecified => None,
            value => Some(review_state_name(value)),
        };
        type Row = (
            Uuid,
            Uuid,
            String,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        );
        let rows = sqlx::query_as::<_, Row>(
            "SELECT id, evaluation_id, state, assigned_to, created_at, updated_at FROM trust_risk_review_cases WHERE ($1::text IS NULL OR state = $1) ORDER BY created_at, id LIMIT $2",
        ).bind(state).bind(i64::from(request.limit.clamp(1, 100))).fetch_all(&self.state.db).await.map_err(unavailable)?;
        Ok(Response::new(pb::ListReviewCasesResponse {
            cases: rows.into_iter().map(review_case_row).collect(),
        }))
    }

    async fn transition_review_case(
        &self,
        request: Request<pb::TransitionReviewCaseRequest>,
    ) -> Result<Response<pb::ReviewCase>, Status> {
        let (metadata, request) = parts(request);
        let operator = authorize(
            &metadata,
            &self.state,
            request.operator.as_ref(),
            "review:write",
        )?;
        let id = uuid(&request.review_case_id)?;
        if let Some(wire_label) = request.resolution_label {
            let label = LabelAssertion::try_from(wire_label)
                .map_err(|_| Status::invalid_argument("resolution label is invalid"))?;
            if label.review_case_id() != Some(id) || label.actor() != Some(operator.actor.as_str())
            {
                return Err(Status::invalid_argument(
                    "resolution label does not match review",
                ));
            }
            labels_db::persist_label(
                &self.state.db,
                &label,
                self.state.config.retention.labels_days,
            )
            .await
            .map_err(map_label)?;
        }
        let target = review_state(
            pb::ReviewCaseState::try_from(request.target_state)
                .map_err(|_| Status::invalid_argument("target state is invalid"))?,
        )?;
        let case = review_db::transition(
            &self.state.db,
            id,
            target,
            request.assign_to,
            &operator.actor,
            &operator.reason,
        )
        .await
        .map_err(map_review)?;
        Ok(Response::new(review_case(case)))
    }

    async fn stage_rule_set(
        &self,
        request: Request<pb::StageRuleSetRequest>,
    ) -> Result<Response<pb::RuleSetReceipt>, Status> {
        let (metadata, request) = parts(request);
        let operator = authorize(
            &metadata,
            &self.state,
            request.operator.as_ref(),
            "rules:write",
        )?;
        if request.canonical_json.len() > 256 * 1024 {
            return Err(Status::resource_exhausted("rule set exceeds size budget"));
        }
        rules_db::stage(
            &self.state.db,
            &request.version,
            &request.canonical_json,
            &operator.actor,
            &operator.reason,
            self.state.config.retention.audit_days,
        )
        .await
        .map(rule_receipt)
        .map(Response::new)
        .map_err(map_rule)
    }

    async fn activate_rule_set(
        &self,
        request: Request<pb::ActivateRuleSetRequest>,
    ) -> Result<Response<pb::RuleSetReceipt>, Status> {
        let (metadata, request) = parts(request);
        let operator = authorize(
            &metadata,
            &self.state,
            request.operator.as_ref(),
            "rules:write",
        )?;
        rules_db::activate(
            &self.state.db,
            &request.version,
            &operator.actor,
            &operator.reason,
            self.state.config.retention.audit_days,
        )
        .await
        .map(rule_receipt)
        .map(Response::new)
        .map_err(map_rule)
    }

    async fn rollback_rule_set(
        &self,
        request: Request<pb::RollbackRuleSetRequest>,
    ) -> Result<Response<pb::RuleSetReceipt>, Status> {
        let (metadata, request) = parts(request);
        let operator = authorize(
            &metadata,
            &self.state,
            request.operator.as_ref(),
            "rules:write",
        )?;
        rules_db::rollback(
            &self.state.db,
            &request.version,
            &operator.actor,
            &operator.reason,
            self.state.config.retention.audit_days,
        )
        .await
        .map(rule_receipt)
        .map(Response::new)
        .map_err(map_rule)
    }
}

fn parts<T>(request: Request<T>) -> (tonic::metadata::MetadataMap, T) {
    (request.metadata().clone(), request.into_inner())
}

fn authorize<'a>(
    metadata: &tonic::metadata::MetadataMap,
    state: &TrustRiskState,
    value: Option<&'a pb::OperatorContext>,
    permission: &str,
) -> Result<&'a pb::OperatorContext, Status> {
    let value = value.ok_or_else(|| Status::invalid_argument("operator context is required"))?;
    if value.caller.len() < 3
        || value.actor.len() < 3
        || value.reason.trim().len() < 3
        || value.reason.len() > 300
    {
        return Err(Status::invalid_argument("operator context is invalid"));
    }
    auth::operator(metadata, &state.config, &value.actor, permission)?;
    Ok(value)
}

#[cfg(test)]
#[path = "trust_risk.operations.tests.rs"]
mod tests;
