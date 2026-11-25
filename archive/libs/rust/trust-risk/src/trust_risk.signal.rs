use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    attribute::{Attribute, convert_attributes},
    proto::nvbes::trust_risk::v1 as pb,
};

const MAX_NAMESPACE_LEN: usize = 80;
const MAX_OPAQUE_ID_LEN: usize = 200;
const MAX_ATTRIBUTES: usize = 32;
const MAX_SUBJECTS: usize = 16;

#[derive(Debug, Clone, PartialEq)]
pub struct RiskSignal {
    id: Uuid,
    producer: String,
    kind: String,
    occurred_at: DateTime<Utc>,
    partition_key: String,
    subjects: Vec<SubjectReference>,
    attributes: BTreeMap<String, Attribute>,
    scope: pb::DataScope,
}

impl RiskSignal {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn producer(&self) -> &str {
        &self.producer
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    pub fn partition_key(&self) -> &str {
        &self.partition_key
    }

    pub fn subjects(&self) -> &[SubjectReference] {
        &self.subjects
    }

    pub fn attributes(&self) -> &BTreeMap<String, Attribute> {
        &self.attributes
    }

    pub fn scope(&self) -> pb::DataScope {
        self.scope
    }
}

impl TryFrom<pb::RiskSignal> for RiskSignal {
    type Error = SignalError;

    fn try_from(value: pb::RiskSignal) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&value.signal_id).map_err(|_| SignalError::InvalidSignalId)?;
        if value.schema_version != 1 {
            return Err(SignalError::UnsupportedSchema);
        }
        let producer =
            validated_name(&value.producer, 3, 80, false).ok_or(SignalError::InvalidProducer)?;
        let kind = validated_name(&value.signal_kind, 3, 100, true)
            .filter(|kind| kind.contains('.'))
            .ok_or(SignalError::InvalidSignalKind)?;
        let occurred_at = value
            .occurred_at
            .and_then(|timestamp| {
                DateTime::from_timestamp(timestamp.seconds, timestamp.nanos as u32)
            })
            .ok_or(SignalError::InvalidTimestamp)?;
        let partition_key = value.partition_key.trim();
        if partition_key.len() < 3
            || partition_key.len() > 200
            || !partition_key.bytes().all(is_partition_byte)
        {
            return Err(SignalError::InvalidPartitionKey);
        }
        if value.subjects.is_empty() || value.subjects.len() > MAX_SUBJECTS {
            return Err(SignalError::InvalidSubjects);
        }
        let subjects = value
            .subjects
            .into_iter()
            .map(SubjectReference::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        let scope = pb::DataScope::try_from(value.scope).map_err(|_| SignalError::InvalidScope)?;
        if matches!(
            scope,
            pb::DataScope::Unspecified | pb::DataScope::GlobalDerived
        ) {
            return Err(SignalError::InvalidScope);
        }
        if value.attributes.len() > MAX_ATTRIBUTES {
            return Err(SignalError::InvalidAttribute);
        }
        let family = kind
            .split('.')
            .next()
            .ok_or(SignalError::InvalidSignalKind)?;
        let attributes = convert_attributes(family, value.attributes)?;

        Ok(Self {
            id,
            producer,
            kind,
            occurred_at,
            partition_key: partition_key.to_string(),
            subjects,
            attributes,
            scope,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectReference {
    kind: pb::SubjectKind,
    namespace: String,
    opaque_id: String,
    scope: pb::DataScope,
    tenant_id: Option<Uuid>,
}

impl SubjectReference {
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn kind(&self) -> pb::SubjectKind {
        self.kind
    }

    pub fn opaque_id(&self) -> &str {
        &self.opaque_id
    }

    pub fn scope(&self) -> pb::DataScope {
        self.scope
    }

    pub fn tenant_id(&self) -> Option<Uuid> {
        self.tenant_id
    }
}

impl TryFrom<pb::SubjectReference> for SubjectReference {
    type Error = SignalError;

    fn try_from(value: pb::SubjectReference) -> Result<Self, Self::Error> {
        let kind =
            pb::SubjectKind::try_from(value.kind).map_err(|_| SignalError::InvalidSubjectKind)?;
        if kind == pb::SubjectKind::Unspecified {
            return Err(SignalError::InvalidSubjectKind);
        }
        let scope = pb::DataScope::try_from(value.scope).map_err(|_| SignalError::InvalidScope)?;
        if matches!(
            scope,
            pb::DataScope::Unspecified | pb::DataScope::GlobalDerived
        ) {
            return Err(SignalError::InvalidScope);
        }
        let namespace = value.namespace.trim();
        if namespace.len() < 3
            || namespace.len() > MAX_NAMESPACE_LEN
            || !namespace.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b'-' | b'_')
            })
        {
            return Err(SignalError::InvalidNamespace);
        }
        let opaque_id = value.opaque_id.trim();
        if opaque_id.contains('@') {
            return Err(SignalError::ForbiddenSubject);
        }
        if opaque_id.len() < 8
            || opaque_id.len() > MAX_OPAQUE_ID_LEN
            || !opaque_id.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
            })
        {
            return Err(SignalError::InvalidSubject);
        }
        let tenant_id = value
            .tenant_id
            .filter(|tenant| !tenant.trim().is_empty())
            .map(|tenant| Uuid::parse_str(&tenant).map_err(|_| SignalError::InvalidTenant))
            .transpose()?;
        if scope == pb::DataScope::Tenant && tenant_id.is_none() {
            return Err(SignalError::InvalidTenant);
        }

        Ok(Self {
            kind,
            namespace: namespace.to_string(),
            opaque_id: opaque_id.to_string(),
            scope,
            tenant_id,
        })
    }
}

pub(crate) fn validated_name(
    value: &str,
    min_len: usize,
    max_len: usize,
    require_lowercase: bool,
) -> Option<String> {
    let value = value.trim();
    let valid = value.len() >= min_len
        && value.len() <= max_len
        && value.bytes().all(|byte| {
            byte.is_ascii_digit()
                || byte.is_ascii_lowercase()
                || (!require_lowercase && byte.is_ascii_uppercase())
                || b"._:-".contains(&byte)
        });
    valid.then(|| value.to_string())
}

fn is_partition_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"._:-".contains(&byte)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SignalError {
    #[error("signal identifier is invalid")]
    InvalidSignalId,
    #[error("signal schema version is unsupported")]
    UnsupportedSchema,
    #[error("signal producer is invalid")]
    InvalidProducer,
    #[error("signal kind is invalid")]
    InvalidSignalKind,
    #[error("signal timestamp is invalid")]
    InvalidTimestamp,
    #[error("signal partition key is invalid")]
    InvalidPartitionKey,
    #[error("signal subject collection is invalid")]
    InvalidSubjects,
    #[error("signal attribute is not allowed or is invalid")]
    InvalidAttribute,
    #[error("subject kind is invalid")]
    InvalidSubjectKind,
    #[error("subject scope is invalid")]
    InvalidScope,
    #[error("subject namespace is invalid")]
    InvalidNamespace,
    #[error("subject identifier is invalid")]
    InvalidSubject,
    #[error("raw identifying subject data is forbidden")]
    ForbiddenSubject,
    #[error("tenant identifier is invalid")]
    InvalidTenant,
}
