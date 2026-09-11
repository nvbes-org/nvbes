use super::{OAuthError, ProtocolError, limits};
use crate::{
    oauth::{clients::ClientRegistry, resources::ResourceServers},
    rate_limits::RateLimiter,
    tokens::TokenService,
};
use axum::{
    Extension, Form, Json, Router,
    extract::{DefaultBodyLimit, RawQuery, Request, State, rejection::FormRejection},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
};
use sqlx::PgPool;
use std::{sync::Arc, time::Duration};

#[derive(Clone)]
struct IntrospectionState {
    db: PgPool,
    clients: Arc<ClientRegistry>,
    tokens: Arc<TokenService>,
}
#[derive(Clone)]
struct Audience(String);

pub fn router(
    db: PgPool,
    clients: Arc<ClientRegistry>,
    tokens: Arc<TokenService>,
    resources: Arc<ResourceServers>,
    limiter: RateLimiter,
) -> Router {
    Router::new()
        .route("/oauth/introspect", post(introspect))
        .layer(DefaultBodyLimit::max(20_480))
        .route_layer(middleware::from_fn_with_state(resources, authenticate))
        .route_layer(middleware::from_fn_with_state(
            limits::SourceLimit {
                db: db.clone(),
                limiter,
                browser: None,
            },
            limits::protect_source,
        ))
        .with_state(IntrospectionState {
            db,
            clients,
            tokens,
        })
}

async fn authenticate(
    State(resources): State<Arc<ResourceServers>>,
    mut request: Request,
    next: Next,
) -> Response {
    let mut response = match resources.authenticate(request.headers()) {
        Some(audience) => {
            request
                .extensions_mut()
                .insert(Audience(audience.to_owned()));
            next.run(request).await
        }
        None => (
            StatusCode::UNAUTHORIZED,
            [("www-authenticate", "Basic realm=\"identity-introspection\"")],
            Json(serde_json::json!({"error":"invalid_client"})),
        )
            .into_response(),
    };
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().unwrap());
    response
        .headers_mut()
        .insert("pragma", "no-cache".parse().unwrap());
    response
}

async fn introspect(
    State(state): State<IntrospectionState>,
    Extension(audience): Extension<Audience>,
    RawQuery(query): RawQuery,
    form: Result<Form<Vec<(String, String)>>, FormRejection>,
) -> Result<Response, ProtocolError> {
    let invalid = || ProtocolError::OAuth(OAuthError::InvalidRequest);
    if query.is_some() {
        return Err(invalid());
    }
    let Form(fields) = form.map_err(|_| invalid())?;
    let mut token = None;
    let mut hint = false;
    for (key, value) in fields {
        match key.as_str() {
            "token" if token.is_none() && !value.is_empty() && value.len() <= 16_384 => {
                token = Some(value)
            }
            "token_type_hint" if !hint && value.len() <= 128 => hint = true,
            _ => return Err(invalid()),
        }
    }
    let token = token.ok_or_else(invalid)?;
    let claims = tokio::time::timeout(
        Duration::from_secs(2),
        state
            .tokens
            .introspect(&state.db, &state.clients, &token, &audience.0),
    )
    .await
    .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?
    .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    let value = match claims {
        None => serde_json::json!({"active":false}),
        Some(claims) => {
            let mut value = serde_json::json!({"active":true,"sub":claims.sub,"aud":claims.aud,"iss":claims.iss,"client_id":claims.client_id,"scope":claims.scope,"exp":claims.exp,"iat":claims.iat,"nbf":claims.nbf,"jti":claims.jti,"sid":claims.sid,"grant_id":claims.grant_id,"token_type":if claims.cnf.is_some() {"DPoP"} else {"Bearer"}});
            if let Some(confirmation) = claims.cnf {
                value["cnf"] = serde_json::to_value(confirmation)
                    .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
            }
            value
        }
    };
    Ok(Json(value).into_response())
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.oauth.http.introspection.tests.rs"]
mod tests;
