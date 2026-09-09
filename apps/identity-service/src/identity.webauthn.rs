use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengePurpose {
    Registration,
    Authentication,
    StepUp,
}

impl ChallengePurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Registration => "registration",
            Self::Authentication => "authentication",
            Self::StepUp => "step_up",
        }
    }
}

pub fn challenge_is_live(expires_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    expires_at > now
}

/// WebAuthn authenticators may report zero permanently. Once a non-zero
/// counter has been stored, a decrease is a rollback signal and is rejected.
pub fn next_sign_count(previous: u32, reported: u32) -> Result<u32, ()> {
    if previous > 0 && reported <= previous {
        return Err(());
    }
    Ok(reported)
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::{challenge_is_live, next_sign_count};

    #[test]
    fn challenge_expiry_is_strict() {
        let now = Utc::now();
        assert!(challenge_is_live(now + Duration::seconds(1), now));
        assert!(!challenge_is_live(now, now));
    }

    #[test]
    fn signature_counter_allows_zero_but_rejects_rollback() {
        assert_eq!(next_sign_count(0, 0), Ok(0));
        assert_eq!(next_sign_count(0, 4), Ok(4));
        assert_eq!(next_sign_count(4, 5), Ok(5));
        assert_eq!(next_sign_count(4, 4), Err(()));
        assert_eq!(next_sign_count(4, 3), Err(()));
    }
}
