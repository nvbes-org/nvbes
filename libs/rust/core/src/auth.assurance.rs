use super::Aal;

pub const PRIVILEGED_AUTHENTICATION_MAX_AGE_SECONDS: i64 = 15 * 60;

pub fn is_phishing_resistant_method(method: &str) -> bool {
    matches!(method, "webauthn" | "passkey" | "security_key")
}

pub fn has_recent_phishing_resistant_authentication(
    acr: Option<&str>,
    amr: &[String],
    auth_time: Option<i64>,
    now: i64,
    max_age_seconds: i64,
) -> bool {
    let assurance = acr
        .and_then(|value| value.parse::<Aal>().ok())
        .unwrap_or(Aal::Aal1);
    let phishing_resistant = amr
        .iter()
        .any(|method| is_phishing_resistant_method(method));
    let recent = auth_time.is_some_and(|authenticated_at| {
        authenticated_at <= now && now.saturating_sub(authenticated_at) <= max_age_seconds.max(0)
    });

    assurance >= Aal::Aal2 && phishing_resistant && recent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privileged_authentication_requires_assurance_method_and_freshness() {
        let now = 1_800_000_000;
        let webauthn = vec!["webauthn".to_string()];
        let otp = vec!["otp".to_string()];

        assert!(has_recent_phishing_resistant_authentication(
            Some("aal2"),
            &webauthn,
            Some(now),
            now,
            PRIVILEGED_AUTHENTICATION_MAX_AGE_SECONDS,
        ));
        assert!(!has_recent_phishing_resistant_authentication(
            Some("aal1"),
            &webauthn,
            Some(now),
            now,
            PRIVILEGED_AUTHENTICATION_MAX_AGE_SECONDS,
        ));
        assert!(!has_recent_phishing_resistant_authentication(
            Some("aal2"),
            &otp,
            Some(now),
            now,
            PRIVILEGED_AUTHENTICATION_MAX_AGE_SECONDS,
        ));
        assert!(!has_recent_phishing_resistant_authentication(
            Some("aal2"),
            &webauthn,
            Some(now - PRIVILEGED_AUTHENTICATION_MAX_AGE_SECONDS - 1),
            now,
            PRIVILEGED_AUTHENTICATION_MAX_AGE_SECONDS,
        ));
        assert!(!has_recent_phishing_resistant_authentication(
            Some("aal3"),
            &webauthn,
            Some(now + 1),
            now,
            PRIVILEGED_AUTHENTICATION_MAX_AGE_SECONDS,
        ));
    }
}
