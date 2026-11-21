use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::{authentication::Authentication, tokens_error::TokenError};

/// Server registration policy; never read from browser authorization parameters.
#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthenticationPolicy {
    #[default]
    Primary,
    RecentMfa,
    RecentWebauthn,
}

impl AuthenticationPolicy {
    /// A strong policy bounds token lifetime to the original proof's freshness.
    pub(crate) fn deadline(
        self,
        authentication: &Authentication,
        now: DateTime<Utc>,
    ) -> Result<Option<DateTime<Utc>>, TokenError> {
        authentication.validate(now)?;
        if self == Self::Primary {
            return Ok(None);
        }
        let primary = (authentication.primary_amr == "webauthn")
            .then(|| authentication.authenticated_at + Duration::minutes(5));
        let step_up = authentication
            .step_up_method
            .as_deref()
            .filter(|method| {
                *method == "webauthn" || (self == Self::RecentMfa && *method == "totp")
            })
            .and(
                authentication
                    .step_up_at
                    .zip(authentication.step_up_expires_at),
            )
            .map(|(at, until)| (at + Duration::minutes(5)).min(until));
        primary
            .into_iter()
            .chain(step_up)
            .max()
            .filter(|deadline| *deadline > now)
            .map(Some)
            .ok_or(TokenError::InvalidAuthentication)
    }
}

#[cfg(test)]
#[path = "identity.oauth.authentication_policy.tests.rs"]
mod tests;

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.oauth.authentication_policy.database.tests.rs"]
mod database_tests;
