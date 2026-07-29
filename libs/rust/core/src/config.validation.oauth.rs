use super::AppConfig;

pub(super) fn validate_fapi_high_assurance(config: &AppConfig) -> Result<(), String> {
    if !config.fapi_high_assurance_enabled {
        return Ok(());
    }
    if !config.dpop_enabled && !config.mtls_enabled {
        return Err(
            "NVBES_FAPI_HIGH_ASSURANCE_ENABLED requires NVBES_DPOP_ENABLED or NVBES_MTLS_ENABLED"
                .to_string(),
        );
    }
    if config.environment == "production" {
        let digest = config
            .fapi_conformance_evidence_sha256
            .as_deref()
            .unwrap_or_default();
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("Production high-assurance OAuth requires a 64-character \
                 NVBES_FAPI_CONFORMANCE_EVIDENCE_SHA256 digest"
                .to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_assurance_is_disabled_by_default() {
        assert!(validate_fapi_high_assurance(&AppConfig::default()).is_ok());
    }

    #[test]
    fn enabled_profile_requires_sender_constrained_tokens() {
        let config = AppConfig {
            fapi_high_assurance_enabled: true,
            ..AppConfig::default()
        };

        assert!(
            validate_fapi_high_assurance(&config)
                .unwrap_err()
                .contains("NVBES_DPOP_ENABLED or NVBES_MTLS_ENABLED")
        );
    }

    #[test]
    fn production_requires_conformance_evidence_digest() {
        let config = AppConfig {
            environment: "production".to_string(),
            dpop_enabled: true,
            fapi_high_assurance_enabled: true,
            ..AppConfig::default()
        };

        assert!(
            validate_fapi_high_assurance(&config)
                .unwrap_err()
                .contains("NVBES_FAPI_CONFORMANCE_EVIDENCE_SHA256")
        );
    }

    #[test]
    fn production_accepts_well_formed_conformance_evidence_digest() {
        let config = AppConfig {
            environment: "production".to_string(),
            dpop_enabled: true,
            fapi_high_assurance_enabled: true,
            fapi_conformance_evidence_sha256: Some("a".repeat(64)),
            ..AppConfig::default()
        };

        assert!(validate_fapi_high_assurance(&config).is_ok());
    }
}
