use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::tokens_error::TokenError;

/// Only authenticators write session evidence. Protocol callers read a snapshot.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Authentication {
    pub authenticated_at: DateTime<Utc>,
    pub primary_amr: String,
    pub step_up_method: Option<String>,
    pub step_up_at: Option<DateTime<Utc>>,
    pub step_up_expires_at: Option<DateTime<Utc>>,
}

impl Authentication {
    pub fn validate(&self, now: DateTime<Utc>) -> Result<(), TokenError> {
        if self.authenticated_at.timestamp() < 0
            || self.authenticated_at > now
            || !matches!(self.primary_amr.as_str(), "pwd" | "webauthn")
        {
            return Err(TokenError::InvalidAuthentication);
        }
        match (
            &self.step_up_method,
            self.step_up_at,
            self.step_up_expires_at,
        ) {
            (None, None, None) => Ok(()),
            (Some(method), Some(at), Some(until))
                if matches!(method.as_str(), "totp" | "webauthn")
                    && at >= self.authenticated_at
                    && at <= now
                    && until > at =>
            {
                Ok(())
            }
            _ => Err(TokenError::InvalidAuthentication),
        }
    }

    pub fn amr(&self, now: DateTime<Utc>) -> Vec<String> {
        let mut methods = vec![self.primary_amr.clone()];
        if self.step_up_expires_at.is_some_and(|until| until > now)
            && let Some(method) = &self.step_up_method
            && !methods.contains(method)
        {
            methods.push(method.clone());
        }
        methods
    }

    pub fn fresh_step_up(&self, now: DateTime<Utc>) -> Option<(u64, u64)> {
        self.step_up_at
            .zip(self.step_up_expires_at)
            .filter(|(_, until)| *until > now)
            .map(|(at, until)| (at.timestamp() as u64, until.timestamp() as u64))
    }
}
