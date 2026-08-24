use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{proto::nvbes::trust_risk::v1 as pb, signal::validated_name};

#[derive(Debug, Clone, PartialEq)]
pub struct LabelAssertion {
    id: Uuid,
    producer: String,
    evaluation_id: Uuid,
    review_case_id: Option<Uuid>,
    kind: pb::RiskLabelKind,
    source_class: pb::LabelSourceClass,
    source_id: String,
    confidence: f64,
    actor: Option<String>,
    knowledge_at: DateTime<Utc>,
    evidence_reference: Option<String>,
    mapping_version: String,
    corrects_label_id: Option<Uuid>,
}

impl LabelAssertion {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn evaluation_id(&self) -> Uuid {
        self.evaluation_id
    }

    pub fn kind(&self) -> pb::RiskLabelKind {
        self.kind
    }

    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    pub fn source_class(&self) -> pb::LabelSourceClass {
        self.source_class
    }

    pub fn actor(&self) -> Option<&str> {
        self.actor.as_deref()
    }
}

impl TryFrom<pb::RiskLabel> for LabelAssertion {
    type Error = LabelError;

    fn try_from(value: pb::RiskLabel) -> Result<Self, Self::Error> {
        let id = parse_uuid(&value.label_id, LabelError::InvalidLabelId)?;
        if value.schema_version != 1 {
            return Err(LabelError::UnsupportedSchema);
        }
        let producer =
            validated_name(&value.producer, 3, 80, false).ok_or(LabelError::InvalidProducer)?;
        let evaluation_id = parse_uuid(&value.evaluation_id, LabelError::InvalidEvaluationId)?;
        let review_case_id =
            parse_optional_uuid(value.review_case_id, LabelError::InvalidReviewCaseId)?;
        let kind = pb::RiskLabelKind::try_from(value.kind).map_err(|_| LabelError::InvalidKind)?;
        if kind == pb::RiskLabelKind::Unspecified {
            return Err(LabelError::InvalidKind);
        }
        let source_class = pb::LabelSourceClass::try_from(value.source_class)
            .map_err(|_| LabelError::InvalidSourceClass)?;
        if source_class == pb::LabelSourceClass::Unspecified {
            return Err(LabelError::InvalidSourceClass);
        }
        let source_id =
            validated_name(&value.source_id, 3, 120, false).ok_or(LabelError::InvalidSourceId)?;
        if !value.confidence.is_finite() || !(0.0..=1.0).contains(&value.confidence) {
            return Err(LabelError::InvalidConfidence);
        }
        let actor = value
            .actor
            .filter(|actor| !actor.trim().is_empty())
            .map(|actor| validated_name(&actor, 3, 120, false).ok_or(LabelError::InvalidActor))
            .transpose()?;
        if source_class == pb::LabelSourceClass::Human && actor.is_none() {
            return Err(LabelError::HumanActorRequired);
        }
        let knowledge_at = value
            .knowledge_at
            .and_then(|timestamp| {
                DateTime::from_timestamp(timestamp.seconds, timestamp.nanos as u32)
            })
            .ok_or(LabelError::InvalidTimestamp)?;
        let evidence_reference = validated_optional(value.evidence_reference, 200)?;
        let mapping_version = validated_name(&value.mapping_version, 3, 80, false)
            .ok_or(LabelError::InvalidMappingVersion)?;
        let corrects_label_id =
            parse_optional_uuid(value.corrects_label_id, LabelError::InvalidCorrectionId)?;
        if corrects_label_id == Some(id) {
            return Err(LabelError::InvalidCorrectionId);
        }

        Ok(Self {
            id,
            producer,
            evaluation_id,
            review_case_id,
            kind,
            source_class,
            source_id,
            confidence: value.confidence,
            actor,
            knowledge_at,
            evidence_reference,
            mapping_version,
            corrects_label_id,
        })
    }
}

fn parse_uuid(value: &str, error: LabelError) -> Result<Uuid, LabelError> {
    Uuid::parse_str(value).map_err(|_| error)
}

fn parse_optional_uuid(
    value: Option<String>,
    error: LabelError,
) -> Result<Option<Uuid>, LabelError> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(|value| parse_uuid(&value, error))
        .transpose()
}

fn validated_optional(value: Option<String>, max_len: usize) -> Result<Option<String>, LabelError> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            validated_name(&value, 3, max_len, false).ok_or(LabelError::InvalidEvidenceReference)
        })
        .transpose()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LabelError {
    #[error("label identifier is invalid")]
    InvalidLabelId,
    #[error("label schema version is unsupported")]
    UnsupportedSchema,
    #[error("label producer is invalid")]
    InvalidProducer,
    #[error("evaluation identifier is invalid")]
    InvalidEvaluationId,
    #[error("review case identifier is invalid")]
    InvalidReviewCaseId,
    #[error("label kind is invalid")]
    InvalidKind,
    #[error("label source class is invalid")]
    InvalidSourceClass,
    #[error("label source identifier is invalid")]
    InvalidSourceId,
    #[error("label confidence is invalid")]
    InvalidConfidence,
    #[error("human labels require an actor")]
    HumanActorRequired,
    #[error("label actor is invalid")]
    InvalidActor,
    #[error("label knowledge timestamp is invalid")]
    InvalidTimestamp,
    #[error("evidence reference is invalid")]
    InvalidEvidenceReference,
    #[error("mapping version is invalid")]
    InvalidMappingVersion,
    #[error("corrected label identifier is invalid")]
    InvalidCorrectionId,
}
