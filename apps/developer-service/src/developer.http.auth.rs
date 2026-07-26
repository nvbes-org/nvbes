use axum::{
    extract::State,
    http::{HeaderMap, Request, header},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{app::DeveloperAppState, http::error::AppError, identity::IdentityClaims};

#[derive(Debug, Clone)]
pub struct DeveloperAuth {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub display_name: String,
    pub email: String,
    pub access_token: String,
    pub claims: IdentityClaims,
}

pub async fn authenticate(
    State(state): State<DeveloperAppState>,
    headers: HeaderMap,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, AppError> {
    let token = bearer_token(&headers)?;
    let claims = state.identity.introspect(&token, Some(&headers)).await?;
    let auth = authenticated_user(token, claims)?;
    request.extensions_mut().insert(auth);
    Ok(next.run(request).await)
}

fn bearer_token(headers: &HeaderMap) -> Result<String, AppError> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::unauthorized("missing_authorization", "Missing bearer token."))
}

fn authenticated_user(
    access_token: String,
    claims: IdentityClaims,
) -> Result<DeveloperAuth, AppError> {
    if !claims.active {
        return Err(AppError::unauthorized(
            "invalid_token",
            "The access token is inactive.",
        ));
    }
    if claims.network_valid == Some(false) {
        return Err(AppError::forbidden(
            "network_restriction_violated",
            "The current network is not allowed.",
        ));
    }
    if claims.principal_type.as_deref() != Some("user") {
        return Err(AppError::forbidden(
            "user_context_required",
            "Developer Console requires a user principal.",
        ));
    }
    let user_id = claims
        .sub
        .as_deref()
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| AppError::unauthorized("invalid_token", "Invalid token subject."))?;
    let tenant_id = claims.tenant_id.ok_or_else(|| {
        AppError::forbidden(
            "tenant_context_required",
            "Developer Console requires a tenant-scoped token.",
        )
    })?;

    Ok(DeveloperAuth {
        user_id,
        tenant_id,
        display_name: claims.display_name.clone().unwrap_or_default(),
        email: claims.email.clone().unwrap_or_default(),
        access_token,
        claims,
    })
}

#[cfg(test)]
mod tests {
    use super::authenticated_user;
    use crate::identity::IdentityClaims;
    use uuid::Uuid;

    #[test]
    fn authentication_requires_active_tenant_scoped_user() {
        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let auth = authenticated_user(
            "token".to_string(),
            IdentityClaims {
                active: true,
                scope: None,
                client_id: None,
                principal_type: Some("user".to_string()),
                token_type: Some("access".to_string()),
                sub: Some(user_id.to_string()),
                tenant_id: Some(tenant_id),
                organization_id: None,
                workspace_id: None,
                email: None,
                display_name: None,
                acr: None,
                amr: Vec::new(),
                auth_time: None,
                exp: None,
                iat: None,
                nbf: None,
                network_valid: Some(true),
            },
        )
        .expect("valid identity");

        assert_eq!(auth.user_id, user_id);
        assert_eq!(auth.tenant_id, tenant_id);
    }
}
