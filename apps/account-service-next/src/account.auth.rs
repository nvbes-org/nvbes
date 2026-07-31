use axum::{
    extract::{Request, State},
    http::{HeaderMap, header},
    middleware::Next,
    response::Response,
    routing::MethodRouter,
};
use nvbes_identity_sdk::{IdentityAccessTokenClaims, IdentityJwtVerifier, SdkError};
use uuid::Uuid;

use crate::{app::AppState, error::AppError};

pub const ACCOUNT_AUDIENCE: &str = "nvbes-account-service";
pub const PROFILE_READ_SCOPE: &str = "account:profile:read";
pub const PROFILE_WRITE_SCOPE: &str = "account:profile:write";
pub const PREFERENCES_READ_SCOPE: &str = "account:preferences:read";
pub const PREFERENCES_WRITE_SCOPE: &str = "account:preferences:write";
pub const LEGAL_READ_SCOPE: &str = "account:legal:read";
pub const LEGAL_WRITE_SCOPE: &str = "account:legal:write";
pub const EXPORT_SCOPE: &str = "account:export";
pub const DELETE_SCOPE: &str = "account:delete";

#[derive(Clone, Debug)]
pub struct AuthenticatedPrincipal {
    pub principal_id: Uuid,
    pub claims: IdentityAccessTokenClaims,
}

#[derive(Clone)]
struct ProtectedRoute {
    verifier: IdentityJwtVerifier,
    required_scope: &'static str,
}

pub fn protected(
    state: &AppState,
    required_scope: &'static str,
    route: MethodRouter<AppState>,
) -> MethodRouter<AppState> {
    route.layer(axum::middleware::from_fn_with_state(
        ProtectedRoute {
            verifier: state.jwt_verifier.clone(),
            required_scope,
        },
        authenticate,
    ))
}

async fn authenticate(
    State(policy): State<ProtectedRoute>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = bearer_token(&headers)?;
    let claims = policy
        .verifier
        .verify_access_token(token)
        .await
        .map_err(map_verification_error)?;
    let principal_id = authorize_claims(&claims, policy.required_scope)?;
    request.extensions_mut().insert(AuthenticatedPrincipal {
        principal_id,
        claims,
    });
    Ok(next.run(request).await)
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, AppError> {
    let value = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| AppError::unauthorized("invalid_token", "A bearer token is required."))?
        .to_str()
        .map_err(|_| AppError::unauthorized("invalid_token", "Authorization is not valid text."))?;
    let (scheme, token) = value.split_once(' ').ok_or_else(|| {
        AppError::unauthorized("invalid_token", "Authorization must use the Bearer scheme.")
    })?;
    if !scheme.eq_ignore_ascii_case("Bearer")
        || token.is_empty()
        || token.trim() != token
        || token.contains(char::is_whitespace)
    {
        return Err(AppError::unauthorized(
            "invalid_token",
            "Authorization must contain one Bearer token.",
        ));
    }
    Ok(token)
}

fn authorize_claims(
    claims: &IdentityAccessTokenClaims,
    required_scope: &'static str,
) -> Result<Uuid, AppError> {
    claims
        .require_scopes(&[required_scope])
        .map_err(|_| AppError::insufficient_scope(required_scope))?;
    Uuid::parse_str(&claims.sub).map_err(|_| {
        AppError::unauthorized(
            "invalid_token",
            "The Identity token subject must be a UUID principal.",
        )
    })
}

fn map_verification_error(error: SdkError) -> AppError {
    match error {
        SdkError::TokenValidation(_) => {
            AppError::unauthorized("invalid_token", "The Identity access token is invalid.")
        }
        other => {
            tracing::error!(error = %other, "Identity JWKS verification unavailable");
            AppError::service_unavailable(
                "identity_jwks_unavailable",
                "Identity token verification is temporarily unavailable.",
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, header};
    use nvbes_identity_sdk::IdentityAccessTokenClaims;
    use uuid::Uuid;

    use super::{PROFILE_READ_SCOPE, authorize_claims, bearer_token};

    fn claims(scope: &str) -> IdentityAccessTokenClaims {
        IdentityAccessTokenClaims {
            sub: Uuid::nil().to_string(),
            workspace_id: None,
            tenant_id: None,
            organization_id: None,
            token_type: "access".to_string(),
            scope: scope.to_string(),
            amr: vec![],
            client_id: Some("account-web".to_string()),
            iss: "https://identity.example".to_string(),
            aud: "nvbes-account-service".to_string(),
            exp: 2,
            iat: 1,
            act: None,
        }
    }

    #[test]
    fn scopes_are_matched_as_exact_tokens() {
        authorize_claims(&claims(PROFILE_READ_SCOPE), PROFILE_READ_SCOPE)
            .expect("exact scope is accepted");
        let error = authorize_claims(&claims("account:profile"), PROFILE_READ_SCOPE)
            .expect_err("scope prefixes are rejected");
        assert!(format!("{error:?}").contains("insufficient_scope"));
    }

    #[test]
    fn only_one_bearer_token_is_accepted() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer token-value"),
        );
        assert_eq!(bearer_token(&headers).expect("bearer token"), "token-value");

        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer first second"),
        );
        assert!(bearer_token(&headers).is_err());
    }
}
