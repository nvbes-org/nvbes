use crate::http::error::AppError;
use webauthn_rs_core::error::WebauthnError;

pub fn map_webauthn_registration_error(err: WebauthnError) -> AppError {
    let (code, message) = match err {
        WebauthnError::InvalidClientDataType => (
            "webauthn_registration_invalid_client_data",
            "WebAuthn registration client data is invalid.",
        ),
        WebauthnError::MismatchedChallenge => (
            "webauthn_registration_challenge_mismatch",
            "WebAuthn registration challenge does not match.",
        ),
        WebauthnError::InvalidRPOrigin => (
            "webauthn_registration_origin_mismatch",
            "WebAuthn registration origin does not match.",
        ),
        WebauthnError::InvalidRPIDHash => (
            "webauthn_registration_rp_id_mismatch",
            "WebAuthn registration relying party id does not match.",
        ),
        WebauthnError::UserNotPresent => (
            "webauthn_registration_user_not_present",
            "WebAuthn registration requires user presence.",
        ),
        WebauthnError::UserNotVerified => (
            "webauthn_registration_user_not_verified",
            "WebAuthn registration requires user verification.",
        ),
        WebauthnError::AttestationNotSupported => (
            "webauthn_registration_attestation_not_supported",
            "WebAuthn attestation format is not supported.",
        ),
        WebauthnError::AttestationTrustFailure => (
            "webauthn_registration_attestation_trust_failed",
            "WebAuthn attestation trust could not be established.",
        ),
        WebauthnError::AttestationStatementAlgMismatch
        | WebauthnError::AttestationStatementAlgInvalid
        | WebauthnError::AttestationStatementMapInvalid
        | WebauthnError::AttestationStatementResponseMissing
        | WebauthnError::AttestationStatementResponseInvalid
        | WebauthnError::AttestationStatementSigMissing
        | WebauthnError::AttestationStatementSigInvalid
        | WebauthnError::AttestationStatementVerMissing
        | WebauthnError::AttestationStatementVerInvalid
        | WebauthnError::AttestationStatementVerUnsupported
        | WebauthnError::AttestationStatementX5CMissing
        | WebauthnError::AttestationStatementX5CInvalid
        | WebauthnError::AttestationStatementCertInfoMissing
        | WebauthnError::AttestationStatementMissingExtension
        | WebauthnError::AttestationStatementPubAreaMissing
        | WebauthnError::AttestationStatementAlgMissing
        | WebauthnError::MissingAttestationCredentialData
        | WebauthnError::AttestationCertificateRequirementsNotMet
        | WebauthnError::AttestationCertificateTrustStoreEmpty
        | WebauthnError::AttestationLeafCertMissing
        | WebauthnError::AttestationNotVerifiable
        | WebauthnError::AttestationUntrustedAaguid
        | WebauthnError::AttestationFormatMissingAaguid
        | WebauthnError::AttestationCertificateAAGUIDMismatch
        | WebauthnError::AttestationCertificateNonceMismatch
        | WebauthnError::AttestationTpmStInvalid
        | WebauthnError::AttestationTpmPubAreaMismatch
        | WebauthnError::AttestationTpmExtraDataInvalid
        | WebauthnError::AttestationTpmExtraDataMismatch
        | WebauthnError::AttestationTpmPubAreaHashUnknown
        | WebauthnError::AttestationTpmPubAreaHashInvalid
        | WebauthnError::AttestationTpmAttestCertifyInvalid
        | WebauthnError::ParseBase64Failure(_)
        | WebauthnError::ParseCBORFailure(_)
        | WebauthnError::ParseJSONFailure(_)
        | WebauthnError::ParseNOMFailure
        | WebauthnError::ParseInsufficientBytesAvailable
        | WebauthnError::OpenSSLError(_)
        | WebauthnError::OpenSSLErrorNoCurveName
        | WebauthnError::COSEKeyInvalidCBORValue
        | WebauthnError::COSEKeyInvalidType
        | WebauthnError::COSEKeyEDUnsupported
        | WebauthnError::COSEKeyECDSAXYInvalid
        | WebauthnError::COSEKeyRSANEInvalid
        | WebauthnError::COSEKeyECDSAInvalidCurve
        | WebauthnError::COSEKeyEDDSAXInvalid
        | WebauthnError::COSEKeyEDDSAInvalidCurve
        | WebauthnError::COSEKeyInvalidAlgorithm
        | WebauthnError::CredentialMayNotBeHardwareBound
        | WebauthnError::CredentialInsecureCryptography => (
            "webauthn_registration_attestation_invalid",
            "WebAuthn registration attestation is invalid.",
        ),
        _ => (
            "webauthn_registration_failed",
            "WebAuthn registration failed.",
        ),
    };

    AppError::forbidden(code, message)
}

#[cfg(test)]
mod tests {
    use super::map_webauthn_registration_error;
    use webauthn_rs_core::error::WebauthnError;

    #[test]
    fn maps_challenge_mismatch_to_specific_error() {
        let error = map_webauthn_registration_error(WebauthnError::MismatchedChallenge);

        assert_eq!(error.code, "webauthn_registration_challenge_mismatch");
        assert_eq!(error.status, axum::http::StatusCode::FORBIDDEN);
    }
}
