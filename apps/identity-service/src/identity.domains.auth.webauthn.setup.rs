use nvbes_core::config::AppConfig;
use reqwest::Url;
use webauthn_rs::prelude::WebauthnBuilder;

use crate::http::error::AppError;

pub fn build_webauthn(config: &AppConfig) -> Result<webauthn_rs::Webauthn, AppError> {
    let origin = Url::parse(&config.webauthn_rp_origin).map_err(|_| {
        AppError::internal(
            "invalid_webauthn_origin",
            "NVBES_WEBAUTHN_RP_ORIGIN must be a valid URL.",
        )
    })?;

    WebauthnBuilder::new(&config.webauthn_rp_id, &origin)
        .map_err(|_| {
            AppError::internal(
                "webauthn_config_invalid",
                "WebAuthn configuration is invalid.",
            )
        })
        .and_then(|builder| {
            builder.build().map_err(|_| {
                AppError::internal(
                    "webauthn_config_invalid",
                    "WebAuthn configuration is invalid.",
                )
            })
        })
}

#[cfg(test)]
mod tests {
    use super::build_webauthn;
    use nvbes_core::config::AppConfig;

    #[test]
    fn build_webauthn_uses_dedicated_rp_origin() {
        let config = AppConfig {
            web_base_url: "not-a-url".to_string(),
            webauthn_rp_id: "localhost".to_string(),
            webauthn_rp_origin: "http://localhost:3001".to_string(),
            ..AppConfig::default()
        };

        assert!(build_webauthn(&config).is_ok());
    }

    #[test]
    fn build_webauthn_rejects_an_origin_outside_the_rp_id() {
        let config = AppConfig {
            webauthn_rp_id: "nvbes.com".to_string(),
            webauthn_rp_origin: "https://login.example.com".to_string(),
            ..AppConfig::default()
        };

        assert!(build_webauthn(&config).is_err());
    }
}
