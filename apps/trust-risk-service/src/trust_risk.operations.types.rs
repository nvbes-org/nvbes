use std::collections::HashMap;

use nvbes_trust_risk::proto::nvbes::trust_risk::v1 as pb;
use tonic::Status;
use uuid::Uuid;

use crate::{
    labels_db,
    review_db::{self, ReviewState},
    rules_db,
};

pub struct EvaluationParams<'a> {
    pub id: Uuid,
    pub score: i16,
    pub band: &'a str,
    pub recommendation: &'a str,
    pub reasons: Vec<String>,
    pub feature_version: &'a str,
    pub rule_version: &'a str,
    pub evaluated_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

pub fn evaluation(params: EvaluationParams<'_>) -> pb::RiskEvaluation {
    pb::RiskEvaluation {
        evaluation_id: params.id.to_string(),
        score: params.score as u32,
        band: band_value(params.band),
        recommendation: recommendation_value(params.recommendation),
        reasons: params
            .reasons
            .into_iter()
            .map(|code| pb::RiskReason {
                code,
                parameters: HashMap::new(),
            })
            .collect(),
        feature_version: params.feature_version.to_string(),
        rule_set_version: params.rule_version.to_string(),
        evaluated_at: Some(timestamp(params.evaluated_at)),
        expires_at: Some(timestamp(params.expires_at)),
        duplicate: false,
    }
}

pub fn review_case(value: review_db::ReviewCase) -> pb::ReviewCase {
    pb::ReviewCase {
        review_case_id: value.id.to_string(),
        evaluation_id: value.evaluation_id.to_string(),
        state: review_state_value(value.state),
        assigned_to: value.assigned_to,
        created_at: Some(timestamp(value.created_at)),
        updated_at: Some(timestamp(value.updated_at)),
    }
}

pub type ReviewRow = (
    Uuid,
    Uuid,
    String,
    Option<String>,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Utc>,
);

pub fn review_case_row(row: ReviewRow) -> pb::ReviewCase {
    pb::ReviewCase {
        review_case_id: row.0.to_string(),
        evaluation_id: row.1.to_string(),
        state: review_state_string_value(&row.2),
        assigned_to: row.3,
        created_at: Some(timestamp(row.4)),
        updated_at: Some(timestamp(row.5)),
    }
}

pub fn rule_receipt(value: rules_db::RuleSetReceipt) -> pb::RuleSetReceipt {
    pb::RuleSetReceipt {
        version: value.version,
        state: value.state,
        checksum: value.checksum,
        updated_at: Some(timestamp(value.updated_at)),
    }
}

fn timestamp(value: chrono::DateTime<chrono::Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: value.timestamp(),
        nanos: value.timestamp_subsec_nanos() as i32,
    }
}

pub fn uuid(value: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(value).map_err(|_| Status::invalid_argument("identifier is invalid"))
}

pub fn band_value(value: &str) -> i32 {
    match value {
        "low" => pb::RiskBand::Low,
        "elevated" => pb::RiskBand::Elevated,
        "high" => pb::RiskBand::High,
        "critical" => pb::RiskBand::Critical,
        _ => pb::RiskBand::Unspecified,
    }
    .into()
}
pub fn recommendation_value(value: &str) -> i32 {
    match value {
        "allow" => pb::RiskRecommendation::Allow,
        "challenge" => pb::RiskRecommendation::Challenge,
        "review" => pb::RiskRecommendation::Review,
        "deny" => pb::RiskRecommendation::Deny,
        _ => pb::RiskRecommendation::Unspecified,
    }
    .into()
}
pub fn label_kind_name(value: i16) -> Option<&'static str> {
    match value {
        1 => Some("legitimate"),
        2 => Some("confirmed_fraud"),
        3 => Some("bot"),
        4 => Some("account_takeover"),
        5 => Some("chargeback"),
        6 => Some("false_positive"),
        _ => None,
    }
}
pub fn review_state(value: pb::ReviewCaseState) -> Result<ReviewState, Status> {
    match value {
        pb::ReviewCaseState::Open => Ok(ReviewState::Open),
        pb::ReviewCaseState::InReview => Ok(ReviewState::InReview),
        pb::ReviewCaseState::Resolved => Ok(ReviewState::Resolved),
        pb::ReviewCaseState::Inconclusive => Ok(ReviewState::Inconclusive),
        _ => Err(Status::invalid_argument("target state is invalid")),
    }
}
pub fn review_state_name(value: pb::ReviewCaseState) -> &'static str {
    match value {
        pb::ReviewCaseState::Open => "open",
        pb::ReviewCaseState::InReview => "in_review",
        pb::ReviewCaseState::Resolved => "resolved",
        pb::ReviewCaseState::Inconclusive => "inconclusive",
        _ => "",
    }
}
fn review_state_value(value: ReviewState) -> i32 {
    match value {
        ReviewState::Open => pb::ReviewCaseState::Open,
        ReviewState::InReview => pb::ReviewCaseState::InReview,
        ReviewState::Resolved => pb::ReviewCaseState::Resolved,
        ReviewState::Inconclusive => pb::ReviewCaseState::Inconclusive,
    }
    .into()
}
fn review_state_string_value(value: &str) -> i32 {
    match value {
        "open" => pb::ReviewCaseState::Open,
        "in_review" => pb::ReviewCaseState::InReview,
        "resolved" => pb::ReviewCaseState::Resolved,
        "inconclusive" => pb::ReviewCaseState::Inconclusive,
        _ => pb::ReviewCaseState::Unspecified,
    }
    .into()
}

pub fn unavailable(_: sqlx::Error) -> Status {
    Status::unavailable("operations storage unavailable")
}
pub fn map_label(error: labels_db::LabelPersistenceError) -> Status {
    match error {
        labels_db::LabelPersistenceError::Conflict => Status::already_exists("label conflicts"),
        labels_db::LabelPersistenceError::InvalidCorrection => {
            Status::failed_precondition("correction target is invalid")
        }
        _ => Status::unavailable("label storage unavailable"),
    }
}
pub fn map_review(error: review_db::ReviewError) -> Status {
    match error {
        review_db::ReviewError::NotFound => Status::not_found("review not found"),
        review_db::ReviewError::InvalidTransition | review_db::ReviewError::InvalidInput => {
            Status::failed_precondition("review transition is invalid")
        }
        _ => Status::unavailable("review storage unavailable"),
    }
}
pub fn map_rule(error: rules_db::RuleOperationError) -> Status {
    match error {
        rules_db::RuleOperationError::InvalidInput | rules_db::RuleOperationError::InvalidRules => {
            Status::invalid_argument("rule request is invalid")
        }
        rules_db::RuleOperationError::Conflict => Status::already_exists("rule set exists"),
        rules_db::RuleOperationError::NotFound => Status::not_found("rule set not found"),
        rules_db::RuleOperationError::InvalidState | rules_db::RuleOperationError::DualControl => {
            Status::failed_precondition("rule operation is not permitted")
        }
        _ => Status::unavailable("rule storage unavailable"),
    }
}
