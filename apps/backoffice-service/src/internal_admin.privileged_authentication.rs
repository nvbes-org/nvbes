use std::time::Duration;

use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, Request, header},
    middleware::Next,
    response::Response,
};
use base64::Engine;
use tonic::{
    Code, Request as GrpcRequest,
    metadata::MetadataValue,
    transport::{Channel, Endpoint},
};
use uuid::Uuid;

use crate::app::AppState;
use crate::error::AppError;
use crate::grpc_pb::nvbes::identity::internal::v1::{
    IntrospectAccessTokenRequest, IntrospectAccessTokenResponse,
    identity_internal_service_client::IdentityInternalServiceClient,
};
use crate::observability::record_guard_rejection;

const ACCOUNT_GRPC_ENDPOINT_ENV: &str = "NVBES_ACCOUNT_GRPC_ENDPOINT";
const ACCOUNT_CLIENT_ID_ENV: &str = "NVBES_BACKOFFICE_ACCOUNT_CLIENT_ID";
const ACCOUNT_CLIENT_SECRET_ENV: &str = "NVBES_BACKOFFICE_ACCOUNT_CLIENT_SECRET";
const INTROSPECTION_TIMEOUT: Duration = Duration::from_secs(5);
const ACTOR_HEADER: &str = "x-nvbes-actor-principal-id";
const ACR_HEADER: &str = "x-nvbes-authentication-assurance";
const AMR_HEADER: &str = "x-nvbes-authentication-methods";
const AUTH_TIME_HEADER: &str = "x-nvbes-authenticated-at";
const AUTH_EVENT_HEADER: &str = "x-nvbes-authentication-event-id";

#[derive(Clone)]
pub(crate) struct PrivilegedIdentityClient {
    client: IdentityInternalServiceClient<Channel>,
    authorization: MetadataValue<tonic::metadata::Ascii>,
}

#[derive(Debug, Clone)]
struct IdentityClaims {
    active: bool,
    principal_type: Option<String>,
    sub: Option<String>,
    acr: Option<String>,
    amr: Vec<String>,
    auth_time: Option<i64>,
    sid: Option<String>,
    network_valid: Option<bool>,
}

impl PrivilegedIdentityClient {
    pub(crate) fn from_environment(environment: &str) -> Result<Self, String> {
        let development = environment == "development" || environment == "test";
        let required = |name: &str, development_value: &str| {
            std::env::var(name).or_else(|_| {
                development
                    .then(|| development_value.to_string())
                    .ok_or(std::env::VarError::NotPresent)
            })
        };
        let endpoint = std::env::var(ACCOUNT_GRPC_ENDPOINT_ENV)
            .unwrap_or_else(|_| "http://127.0.0.1:4010".to_string());
        let client_id = required(ACCOUNT_CLIENT_ID_ENV, "backoffice-service")
            .map_err(|_| format!("{ACCOUNT_CLIENT_ID_ENV} is required outside development"))?;
        let client_secret = required(
            ACCOUNT_CLIENT_SECRET_ENV,
            "development-backoffice-introspection-secret",
        )
        .map_err(|_| format!("{ACCOUNT_CLIENT_SECRET_ENV} is required outside development"))?;
        let channel = Endpoint::from_shared(endpoint)
            .map_err(|error| format!("{ACCOUNT_GRPC_ENDPOINT_ENV} is invalid: {error}"))?
            .connect_lazy();
        let encoded = base64::engine::general_purpose::STANDARD
            .encode(format!("{client_id}:{client_secret}"));
        let authorization = format!("Basic {encoded}")
            .parse()
            .map_err(|_| "Backoffice Identity credential contains invalid metadata".to_string())?;

        Ok(Self {
            client: IdentityInternalServiceClient::new(channel),
            authorization,
        })
    }

    async fn introspect(
        &self,
        token: &str,
        incoming_headers: &HeaderMap,
    ) -> Result<IdentityClaims, AppError> {
        let client_ip =
            nvbes_core::http::client_ip::client_ip(incoming_headers).unwrap_or_default();
        let mut request = GrpcRequest::new(IntrospectAccessTokenRequest {
            access_token: token.to_string(),
            client_ip,
        });
        request
            .metadata_mut()
            .insert("authorization", self.authorization.clone());
        request.set_timeout(INTROSPECTION_TIMEOUT);
        let response = self
            .client
            .clone()
            .introspect_access_token(request)
            .await
            .map_err(grpc_error)?
            .into_inner();
        Ok(claims_from_response(response))
    }
}

fn claims_from_response(response: IntrospectAccessTokenResponse) -> IdentityClaims {
    IdentityClaims {
        active: response.active,
        principal_type: response.principal_type,
        sub: response.sub,
        acr: response.acr,
        amr: response.amr,
        auth_time: response.auth_time,
        sid: response.sid,
        network_valid: response.network_valid,
    }
}

fn grpc_error(error: tonic::Status) -> AppError {
    match error.code() {
        Code::Unauthenticated => AppError::unauthorized(
            "identity_introspection_rejected",
            "Identity rejected the Backoffice access token.",
        ),
        Code::PermissionDenied => AppError::forbidden(
            "identity_introspection_forbidden",
            "Identity forbids Backoffice token introspection.",
        ),
        _ => AppError::internal("identity_introspection_failed", error.to_string()),
    }
}

pub(crate) async fn privileged_authentication_guard(
    State(state): State<AppState>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, AppError> {
    let token = bearer_token(request.headers())?;
    let claims = state
        .privileged_identity
        .introspect(token, request.headers())
        .await?;
    let verified = require_privileged_authentication(&claims, chrono::Utc::now().timestamp())?;
    tracing::info!(
        security_event = "privileged_authentication_enforced",
        privileged_surface = "backoffice_and_support",
        principal_id = %verified.principal_id,
        authentication_event_id = %verified.authentication_event_id,
        acr = %verified.acr,
        amr = %verified.amr.join(","),
        auth_time = verified.auth_time,
        "accepted privileged Backoffice request"
    );
    insert_verified_headers(request.headers_mut(), &verified)?;
    Ok(next.run(request).await)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VerifiedPrivilegedAuthentication {
    principal_id: Uuid,
    acr: String,
    amr: Vec<String>,
    auth_time: i64,
    authentication_event_id: String,
}

fn require_privileged_authentication(
    claims: &IdentityClaims,
    now: i64,
) -> Result<VerifiedPrivilegedAuthentication, AppError> {
    let principal_id = claims
        .sub
        .as_deref()
        .and_then(|value| Uuid::parse_str(value).ok());
    let valid = claims.active
        && claims.principal_type.as_deref() == Some("user")
        && claims.network_valid != Some(false)
        && principal_id.is_some()
        && claims
            .sid
            .as_deref()
            .is_some_and(|sid| !sid.trim().is_empty())
        && nvbes_core::auth::has_recent_phishing_resistant_authentication(
            claims.acr.as_deref(),
            &claims.amr,
            claims.auth_time,
            now,
            nvbes_core::auth::PRIVILEGED_AUTHENTICATION_MAX_AGE_SECONDS,
        );
    if valid {
        return Ok(VerifiedPrivilegedAuthentication {
            principal_id: principal_id.expect("validated principal id"),
            acr: claims.acr.clone().expect("validated AAL2 assurance"),
            amr: claims.amr.clone(),
            auth_time: claims.auth_time.expect("validated authentication time"),
            authentication_event_id: claims.sid.clone().expect("validated session id"),
        });
    }

    record_guard_rejection("authentication", "phishing_resistant_required");
    Err(AppError::forbidden(
        "phishing_resistant_authentication_required",
        "Backoffice access requires a recent AAL2 passkey or hardware security-key authentication.",
    ))
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, AppError> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::unauthorized("missing_authorization", "Missing bearer token."))
}

fn insert_verified_headers(
    headers: &mut HeaderMap,
    verified: &VerifiedPrivilegedAuthentication,
) -> Result<(), AppError> {
    let values = [
        (ACTOR_HEADER, verified.principal_id.to_string()),
        (ACR_HEADER, verified.acr.clone()),
        (AMR_HEADER, verified.amr.join(",")),
        (AUTH_TIME_HEADER, verified.auth_time.to_string()),
        (AUTH_EVENT_HEADER, verified.authentication_event_id.clone()),
    ];
    for (name, value) in values {
        headers.insert(
            name,
            HeaderValue::from_str(&value).map_err(|_| {
                AppError::internal(
                    "verified_authentication_header_invalid",
                    "Identity returned an invalid privileged authentication value.",
                )
            })?,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(acr: &str, amr: &[&str], auth_time: i64) -> IdentityClaims {
        IdentityClaims {
            active: true,
            principal_type: Some("user".to_string()),
            sub: Some(Uuid::nil().to_string()),
            acr: Some(acr.to_string()),
            amr: amr.iter().map(|method| (*method).to_string()).collect(),
            auth_time: Some(auth_time),
            sid: Some("authn-event-01".to_string()),
            network_valid: Some(true),
        }
    }

    #[test]
    fn backoffice_requires_recent_phishing_resistant_authentication() {
        let now = 1_800_000_000;

        assert!(
            require_privileged_authentication(&claims("aal2", &["pwd", "webauthn"], now), now)
                .is_ok()
        );
        assert!(require_privileged_authentication(&claims("aal2", &["otp"], now), now).is_err());
        assert!(
            require_privileged_authentication(&claims("aal1", &["webauthn"], now), now).is_err()
        );
        assert!(
            require_privileged_authentication(
                &claims(
                    "aal2",
                    &["security_key"],
                    now - nvbes_core::auth::PRIVILEGED_AUTHENTICATION_MAX_AGE_SECONDS - 1,
                ),
                now,
            )
            .is_err()
        );
    }

    #[test]
    fn backoffice_rejects_a_session_without_an_auditable_identifier() {
        let now = 1_800_000_000;
        let mut identity = claims("aal2", &["webauthn"], now);
        identity.sid = None;

        assert!(require_privileged_authentication(&identity, now).is_err());
    }
}
