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
}

impl Category {
    fn policy(self) -> (&'static str, i32, i32) {
        match self {
            Self::LoginAccount => ("login_account", 5, 600),
            Self::LoginSource => ("login_source", 30, 60),
            Self::ProtocolSource => ("protocol_source", 120, 60),
        }
    }
}

/// Share the same stable secret across replicas. No raw IP, email or unkeyed
/// identifier hash is stored. 3 * 4096 slots is the maximum table cardinality.
#[derive(Clone)]
pub struct RateLimiter {
    key: [u8; 32],
}

impl RateLimiter {
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
