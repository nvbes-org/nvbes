use axum::{Json, extract::State};
use nvbes_core::http::error::ErrorEnvelope;
use serde::Serialize;

use crate::{app::AppState, http::error::AppError};

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct ChangePasswordWellKnownResponse {
    change_password: String,
}

#[utoipa::path(
    get,
    path = "/.well-known/change-password",
    tag = "auth",
    responses(
        (status = 200, description = "Password change URL", body = ChangePasswordWellKnownResponse),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn change_password_well_known(
    State(state): State<AppState>,
) -> Result<Json<ChangePasswordWellKnownResponse>, AppError> {
    Ok(Json(ChangePasswordWellKnownResponse {
        change_password: change_password_url(&state.config.web_base_url),
    }))
}

fn change_password_url(web_base_url: &str) -> String {
    format!(
        "{}/account/security/password",
        web_base_url.trim_end_matches('/')
    )
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct GpcWellKnownResponse {
    gpc: bool,
    version: u32,
}

#[utoipa::path(
    get,
    path = "/.well-known/gpc.json",
    tag = "auth",
    responses(
        (status = 200, description = "GPC support status", body = GpcWellKnownResponse),
    ),
)]
pub(crate) async fn gpc_well_known() -> Json<GpcWellKnownResponse> {
    Json(GpcWellKnownResponse {
        gpc: true,
        version: 1,
    })
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct PasskeyEndpointsWellKnownResponse {
    enroll: String,
    manage: String,
}

#[utoipa::path(
    get,
    path = "/.well-known/passkey-endpoints",
    tag = "auth",
    responses(
        (status = 200, description = "Passkey enrollment and management endpoints", body = PasskeyEndpointsWellKnownResponse),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn passkey_endpoints_well_known(
    State(state): State<AppState>,
) -> Result<Json<PasskeyEndpointsWellKnownResponse>, AppError> {
    let base = state.config.web_base_url.trim_end_matches('/').to_string();
    Ok(Json(PasskeyEndpointsWellKnownResponse {
        enroll: format!("{}/account/mfa/passkey/setup", base),
        manage: format!("{}/account/mfa", base),
    }))
}

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct WebAuthnWellKnownResponse {
    origins: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/.well-known/webauthn",
    tag = "auth",
    responses(
        (status = 200, description = "WebAuthn related origins for cross-domain passkey sharing", body = WebAuthnWellKnownResponse),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub(crate) async fn webauthn_well_known(
    State(state): State<AppState>,
) -> Result<Json<WebAuthnWellKnownResponse>, AppError> {
    Ok(Json(WebAuthnWellKnownResponse {
        origins: state.config.webauthn_related_origins.clone(),
    }))
}

pub(crate) async fn security_txt_well_known(
    State(state): State<AppState>,
) -> Result<
    (
        axum::http::StatusCode,
        [(axum::http::HeaderName, &'static str); 1],
        String,
    ),
    AppError,
> {
    let base = state.config.web_base_url.trim_end_matches('/').to_string();
    let contact = match &state.config.security_contact_email {
        Some(email) => format!("mailto:{}", email),
        None => {
            let domain = state
                .config
                .web_base_url
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .split('/')
                .next()
                .unwrap_or("nvbes.fr");
            format!("mailto:security@{}", domain)
        }
    };
    let policy = format!("{}/.well-known/security-policy", base);
    let expires = "2027-06-01T00:00:00.000Z";
    let body = format!(
        "Contact: {}\nPolicy: {}\nExpires: {}\n",
        contact, policy, expires
    );
    Ok((
        axum::http::StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        body,
    ))
}

#[cfg(test)]
mod tests {
    use super::WebAuthnWellKnownResponse;
    use super::change_password_url;

    #[test]
    fn webauthn_well_known_serializes_origins() {
        let response = WebAuthnWellKnownResponse {
            origins: vec!["https://drive.nvbes.io".into()],
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"origins": ["https://drive.nvbes.io"]})
        );
    }

    #[test]
    fn change_password_url_trims_web_base_url() {
        assert_eq!(
            change_password_url("https://identity.example/"),
            "https://identity.example/account/security/password"
        );
    }
}
