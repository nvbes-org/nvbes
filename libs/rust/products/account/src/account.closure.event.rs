use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const ACCOUNT_CLOSURE_REQUESTED_V1: &str = "account.closure.requested.v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AccountClosureRequestedV1 {
    pub event_id: Uuid,
    pub event_type: String,
    pub saga_id: Uuid,
    pub principal_id: Uuid,
    pub requested_at: DateTime<Utc>,
}

impl AccountClosureRequestedV1 {
    pub fn new(
        event_id: Uuid,
        saga_id: Uuid,
        principal_id: Uuid,
        requested_at: DateTime<Utc>,
    ) -> Self {
        Self {
            event_id,
            event_type: ACCOUNT_CLOSURE_REQUESTED_V1.to_string(),
            saga_id,
            principal_id,
            requested_at,
        }
    }

    pub fn validate(&self, now: DateTime<Utc>) -> Result<(), AccountClosureEventError> {
        if self.event_type != ACCOUNT_CLOSURE_REQUESTED_V1 {
            return Err(AccountClosureEventError::UnsupportedVersion);
        }
        if self.requested_at > now + Duration::minutes(5) {
            return Err(AccountClosureEventError::FutureRequest);
        }
        Ok(())
    }

    pub fn fingerprint(&self) -> Result<Vec<u8>, serde_json::Error> {
        Ok(Sha256::digest(serde_json::to_vec(self)?).to_vec())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum AccountClosureEventError {
    #[error("the Account closure event version is unsupported")]
    UnsupportedVersion,
    #[error("the Account closure request time is in the future")]
    FutureRequest,
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use super::{
        ACCOUNT_CLOSURE_REQUESTED_V1, AccountClosureEventError, AccountClosureRequestedV1,
    };

    #[test]
    fn validates_version_and_clock_skew() {
        let now = Utc::now();
        let mut event =
            AccountClosureRequestedV1::new(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), now);
        assert_eq!(event.validate(now), Ok(()));
        event.event_type = "account.closure.requested.v2".to_string();
        assert_eq!(
            event.validate(now),
            Err(AccountClosureEventError::UnsupportedVersion)
        );
        event.event_type = ACCOUNT_CLOSURE_REQUESTED_V1.to_string();
        event.requested_at = now + Duration::minutes(6);
        assert_eq!(
            event.validate(now),
            Err(AccountClosureEventError::FutureRequest)
        );
    }
}
