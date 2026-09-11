use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::PgPool;

#[derive(Debug, thiserror::Error)]
pub enum LimitError {
    #[error("invalid rate limit configuration or subject")]
    Configuration,
    #[error("rate limit exceeded")]
    Exceeded,
    #[error("rate limit persistence unavailable")]
    Database(#[from] sqlx::Error),
}

#[derive(Clone, Copy)]
pub enum Category {
    LoginAccount,
    LoginSource,
    ProtocolSource,
    MfaAccount,
    WebauthnAccount,
    RecoveryAccount,
    RecoverySource,
    RecoveryToken,
}

impl Category {
    fn policy(self) -> (&'static str, i32, i32) {
        match self {
            Self::LoginAccount => ("login_account", 5, 600),
            Self::LoginSource => ("login_source", 30, 60),
            Self::ProtocolSource => ("protocol_source", 120, 60),
            Self::MfaAccount => ("mfa_account", 5, 600),
            Self::WebauthnAccount => ("webauthn_account", 20, 600),
            Self::RecoveryAccount => ("recovery_account", 3, 900),
            Self::RecoverySource => ("recovery_source", 30, 900),
            Self::RecoveryToken => ("recovery_token", 5, 900),
        }
    }

    pub(crate) fn retry_after(self) -> u32 {
        self.policy().2 as u32
    }
}

/// Share the same stable secret across replicas. No raw IP, email or unkeyed
/// identifier hash is stored. 8 * 4096 slots is the maximum table cardinality.
#[derive(Clone)]
pub struct RateLimiter {
    key: [u8; 32],
}

impl RateLimiter {
    pub fn from_base64(value: &str) -> Result<Self, LimitError> {
        use base64::{Engine, engine::general_purpose::STANDARD};
        let key = STANDARD
            .decode(value)
            .map_err(|_| LimitError::Configuration)?;
        Self::new(key.try_into().map_err(|_| LimitError::Configuration)?)
    }

    pub fn new(key: [u8; 32]) -> Result<Self, LimitError> {
        if key == [0; 32] {
            return Err(LimitError::Configuration);
        }
        Ok(Self { key })
    }

    /// Count every admitted attempt, including successful logins. Call with the
    /// normalized identifier and trusted peer address before password hashing.
    /// Fixed-window collisions deny conservatively; no per-subject rows grow.
    pub async fn check(
        &self,
        db: &PgPool,
        category: Category,
        subject: &str,
    ) -> Result<(), LimitError> {
        if subject.is_empty() || subject.len() > 1024 {
            return Err(LimitError::Configuration);
        }
        let (name, maximum, seconds) = category.policy();
        let admitted: Option<i32> = sqlx::query_scalar(
            "INSERT INTO identity_rate_buckets(category,slot,attempts,reset_at) VALUES($1,$2,1,clock_timestamp()+make_interval(secs => $4::integer)) ON CONFLICT(category,slot) DO UPDATE SET attempts=CASE WHEN identity_rate_buckets.reset_at<=clock_timestamp() THEN 1 ELSE identity_rate_buckets.attempts+1 END,reset_at=CASE WHEN identity_rate_buckets.reset_at<=clock_timestamp() THEN clock_timestamp()+make_interval(secs => $4::integer) ELSE identity_rate_buckets.reset_at END WHERE identity_rate_buckets.reset_at<=clock_timestamp() OR identity_rate_buckets.attempts<$3 RETURNING attempts"
        ).bind(name).bind(self.slot(name, subject)).bind(maximum).bind(seconds).fetch_optional(db).await?;
        admitted.map(|_| ()).ok_or(LimitError::Exceeded)
    }

    fn slot(&self, category: &str, subject: &str) -> i32 {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).expect("HMAC accepts 32-byte keys");
        mac.update(category.as_bytes());
        mac.update(&[0]);
        mac.update(subject.as_bytes());
        let bytes = mac.finalize().into_bytes();
        i32::from(u16::from_be_bytes([bytes[0], bytes[1]]) & 4095)
    }
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.rate_limits.tests.rs"]
mod tests;

#[cfg(test)]
mod configuration_tests {
    use super::*;
    use base64::{Engine, engine::general_purpose::STANDARD};

    #[test]
    fn stable_key_requires_exact_nonzero_32_bytes() {
        for invalid in [
            String::new(),
            "not-base64".into(),
            STANDARD.encode([0; 32]),
            STANDARD.encode([1; 31]),
            STANDARD.encode([1; 33]),
        ] {
            assert!(RateLimiter::from_base64(&invalid).is_err());
        }
        let encoded = STANDARD.encode([7; 32]);
        let a = RateLimiter::from_base64(&encoded).unwrap();
        let b = RateLimiter::from_base64(&encoded).unwrap();
        assert_eq!(
            a.slot("login_account", "email@example.invalid"),
            b.slot("login_account", "email@example.invalid")
        );
    }
}
