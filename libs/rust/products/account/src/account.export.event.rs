use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub const ACCOUNT_EXPORT_REQUESTED_V1: &str = "account.export.requested.v1";
pub const ACCOUNT_EXPORT_FRAGMENT_V1: &str = "account-export-fragment.v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AccountExportRequestedV1 {
    pub event_id: Uuid,
    pub event_type: String,
    pub export_id: Uuid,
    pub principal_id: Uuid,
    pub requested_at: DateTime<Utc>,
}

impl AccountExportRequestedV1 {
    pub fn new(
        event_id: Uuid,
        export_id: Uuid,
        principal_id: Uuid,
        requested_at: DateTime<Utc>,
    ) -> Self {
        Self {
            event_id,
            event_type: ACCOUNT_EXPORT_REQUESTED_V1.to_string(),
            export_id,
            principal_id,
            requested_at,
        }
    }

    pub fn validate(&self, now: DateTime<Utc>) -> Result<(), AccountExportContractError> {
        if self.event_type != ACCOUNT_EXPORT_REQUESTED_V1 {
            return Err(AccountExportContractError::UnsupportedVersion);
        }
        if self.requested_at > now + Duration::minutes(5) {
            return Err(AccountExportContractError::FutureRequest);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AccountExportFragmentV1 {
    pub schema_version: String,
    pub participant: String,
    pub principal_id: Uuid,
    pub generated_at: DateTime<Utc>,
    pub data: Value,
}

impl AccountExportFragmentV1 {
    pub fn new(participant: &str, principal_id: Uuid, data: Value) -> Self {
        Self {
            schema_version: ACCOUNT_EXPORT_FRAGMENT_V1.to_string(),
            participant: participant.to_string(),
            principal_id,
            generated_at: Utc::now(),
            data,
        }
    }

    pub fn validate_for(
        &self,
        participant: &str,
        principal_id: Uuid,
    ) -> Result<(), AccountExportContractError> {
        if self.schema_version != ACCOUNT_EXPORT_FRAGMENT_V1 {
            return Err(AccountExportContractError::UnsupportedVersion);
        }
        if self.participant != participant || self.principal_id != principal_id {
            return Err(AccountExportContractError::SubjectMismatch);
        }
        if !self.data.is_object() {
            return Err(AccountExportContractError::InvalidFragment);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum AccountExportContractError {
    #[error("the Account export contract version is unsupported")]
    UnsupportedVersion,
    #[error("the Account export request time is in the future")]
    FutureRequest,
    #[error("the Account export fragment subject does not match the request")]
    SubjectMismatch,
    #[error("the Account export fragment data must be a JSON object")]
    InvalidFragment,
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use serde_json::json;
    use uuid::Uuid;

    use super::{AccountExportContractError, AccountExportFragmentV1, AccountExportRequestedV1};

    #[test]
    fn validates_command_and_fragment_identity() {
        let now = Utc::now();
        let principal_id = Uuid::new_v4();
        let mut command =
            AccountExportRequestedV1::new(Uuid::new_v4(), Uuid::new_v4(), principal_id, now);
        assert_eq!(command.validate(now), Ok(()));
        command.requested_at = now + Duration::minutes(6);
        assert_eq!(
            command.validate(now),
            Err(AccountExportContractError::FutureRequest)
        );

        let fragment = AccountExportFragmentV1::new("cloud", principal_id, json!({}));
        assert_eq!(fragment.validate_for("cloud", principal_id), Ok(()));
        assert_eq!(
            fragment.validate_for("billing", principal_id),
            Err(AccountExportContractError::SubjectMismatch)
        );
    }
}
