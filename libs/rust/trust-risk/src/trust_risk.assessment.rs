use crate::{
    proto::nvbes::trust_risk::v1 as pb,
    signal::{RiskSignal, SignalError, SubjectReference, validated_name},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Assessment {
    producer: String,
    key: String,
    operation_class: String,
    subjects: Vec<SubjectReference>,
    instantaneous_signals: Vec<RiskSignal>,
}

impl Assessment {
    pub fn producer(&self) -> &str {
        &self.producer
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn operation_class(&self) -> &str {
        &self.operation_class
    }

    pub fn subjects(&self) -> &[SubjectReference] {
        &self.subjects
    }

    pub fn instantaneous_signals(&self) -> &[RiskSignal] {
        &self.instantaneous_signals
    }
}

impl TryFrom<pb::AssessRiskRequest> for Assessment {
    type Error = AssessmentError;

    fn try_from(value: pb::AssessRiskRequest) -> Result<Self, Self::Error> {
        let producer = validated_name(&value.producer, 3, 80, false)
            .ok_or(AssessmentError::InvalidProducer)?;
        let key = validated_name(&value.assessment_key, 8, 200, false)
            .ok_or(AssessmentError::InvalidKey)?;
        let operation_class = validated_name(&value.operation_class, 3, 100, true)
            .filter(|class| class.contains('.'))
            .ok_or(AssessmentError::InvalidOperationClass)?;
        if value.subjects.is_empty() || value.subjects.len() > 16 {
            return Err(AssessmentError::InvalidSubjects);
        }
        let subjects = value
            .subjects
            .into_iter()
            .map(SubjectReference::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        if value.instantaneous_signals.len() > 16 {
            return Err(AssessmentError::InvalidSignals);
        }
        let instantaneous_signals = value
            .instantaneous_signals
            .into_iter()
            .map(RiskSignal::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        if instantaneous_signals
            .iter()
            .any(|signal| signal.producer() != producer)
        {
            return Err(AssessmentError::ProducerMismatch);
        }

        Ok(Self {
            producer,
            key,
            operation_class,
            subjects,
            instantaneous_signals,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AssessmentError {
    #[error("assessment producer is invalid")]
    InvalidProducer,
    #[error("assessment key is invalid")]
    InvalidKey,
    #[error("operation class is invalid")]
    InvalidOperationClass,
    #[error("assessment subjects are invalid")]
    InvalidSubjects,
    #[error("assessment signal collection is invalid")]
    InvalidSignals,
    #[error("instantaneous signal producer differs from assessment producer")]
    ProducerMismatch,
    #[error(transparent)]
    Signal(#[from] SignalError),
}
