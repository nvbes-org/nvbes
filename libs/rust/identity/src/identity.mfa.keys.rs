//! Loads the MFA sealing keys shared by every runtime that reads factor or
//! recovery ciphertexts. All of them must load the same keys: a key generated
//! per process would make sealed secrets unreadable after a restart and from
//! the worker.

use base64::{Engine, engine::general_purpose::STANDARD};

use crate::mfa_crypto::MfaCrypto;

pub const ENCRYPTION_KEY: &str = "NVBES_IDENTITY_MFA_ENCRYPTION_KEY";
pub const KEY_VERSION: &str = "NVBES_IDENTITY_MFA_KEY_VERSION";
pub const PREVIOUS_ENCRYPTION_KEY: &str = "NVBES_IDENTITY_MFA_PREVIOUS_ENCRYPTION_KEY";
pub const PREVIOUS_KEY_VERSION: &str = "NVBES_IDENTITY_MFA_PREVIOUS_KEY_VERSION";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MfaKeyError {
    #[error("required configuration is missing: {0}")]
    Missing(&'static str),
    #[error("configuration is invalid: {0}")]
    Invalid(&'static str),
}

/// Raw key material; deliberately not `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct MfaKeys {
    pub active_version: i16,
    pub active_key: [u8; 32],
    pub previous: Option<(i16, [u8; 32])>,
}

impl MfaKeys {
    pub fn from_env() -> Result<Self, MfaKeyError> {
        Self::from_lookup(|name| {
            std::env::var(name)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
    }

    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, MfaKeyError> {
        let active_key = lookup(ENCRYPTION_KEY)
            .ok_or(MfaKeyError::Missing(ENCRYPTION_KEY))
            .and_then(|value| decode_key(ENCRYPTION_KEY, &value))?;
        let active_version = match lookup(KEY_VERSION) {
            Some(value) => parse_version(KEY_VERSION, &value)?,
            None => 1,
        };
        let previous = match (
            lookup(PREVIOUS_ENCRYPTION_KEY),
            lookup(PREVIOUS_KEY_VERSION),
        ) {
            (None, None) => None,
            (Some(key), Some(version)) => {
                let version = parse_version(PREVIOUS_KEY_VERSION, &version)?;
                if version == active_version {
                    return Err(MfaKeyError::Invalid(PREVIOUS_KEY_VERSION));
                }
                Some((version, decode_key(PREVIOUS_ENCRYPTION_KEY, &key)?))
            }
            (Some(_), None) => return Err(MfaKeyError::Missing(PREVIOUS_KEY_VERSION)),
            (None, Some(_)) => return Err(MfaKeyError::Missing(PREVIOUS_ENCRYPTION_KEY)),
        };
        Ok(Self {
            active_version,
            active_key,
            previous,
        })
    }

    pub fn crypto(&self) -> Result<MfaCrypto, MfaKeyError> {
        MfaCrypto::with_rotation(self.active_version, self.active_key, self.previous)
            .map_err(|_| MfaKeyError::Invalid(KEY_VERSION))
    }
}

fn decode_key(name: &'static str, value: &str) -> Result<[u8; 32], MfaKeyError> {
    STANDARD
        .decode(value.trim())
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or(MfaKeyError::Invalid(name))
}

fn parse_version(name: &'static str, value: &str) -> Result<i16, MfaKeyError> {
    value
        .trim()
        .parse::<i16>()
        .ok()
        .filter(|version| *version > 0)
        .ok_or(MfaKeyError::Invalid(name))
}

#[cfg(test)]
#[path = "identity.mfa.keys.tests.rs"]
mod tests;
