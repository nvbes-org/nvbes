use tonic::Status;

use crate::grpc::pb::nvbes::enterprise::v1 as enterprise;

pub fn require(
    authentication: Option<&enterprise::PrivilegedAuthenticationContext>,
) -> Result<(), Status> {
    let authentication = authentication.ok_or_else(|| {
        Status::permission_denied(
            "privileged operation requires a phishing-resistant authentication context",
        )
    })?;
    if authentication.authentication_event_id.trim().is_empty() {
        return Err(Status::permission_denied(
            "privileged authentication event id is required",
        ));
    }
    if !nvbes_core::auth::has_recent_phishing_resistant_authentication(
        Some(authentication.acr.as_str()),
        &authentication.amr,
        Some(authentication.auth_time),
        chrono::Utc::now().timestamp(),
        nvbes_core::auth::PRIVILEGED_AUTHENTICATION_MAX_AGE_SECONDS,
    ) {
        return Err(Status::permission_denied(
            "privileged operation requires a recent AAL2 passkey or hardware security-key authentication",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(
        acr: &str,
        amr: &[&str],
        auth_time: i64,
    ) -> enterprise::PrivilegedAuthenticationContext {
        enterprise::PrivilegedAuthenticationContext {
            acr: acr.to_string(),
            amr: amr.iter().map(|method| (*method).to_string()).collect(),
            auth_time,
            authentication_event_id: "authn-event-01".to_string(),
        }
    }

    #[test]
    fn privileged_operations_require_recent_webauthn() {
        let now = chrono::Utc::now().timestamp();

        assert!(require(Some(&context("aal2", &["webauthn"], now))).is_ok());
        assert!(require(Some(&context("aal2", &["otp"], now))).is_err());
        assert!(require(Some(&context("aal1", &["security_key"], now))).is_err());
        assert!(
            require(Some(&context(
                "aal3",
                &["security_key"],
                now - nvbes_core::auth::PRIVILEGED_AUTHENTICATION_MAX_AGE_SECONDS - 1,
            )))
            .is_err()
        );
        assert!(require(None).is_err());
    }
}
