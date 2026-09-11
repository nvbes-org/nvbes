use super::cors;
use crate::{
    oauth::metadata::provider_metadata, tokens::TokenService, tokens_claims::JsonWebKeySet,
};
use axum::{Json, Router, extract::State, routing::get};
use std::sync::Arc;

#[derive(Clone)]
struct PublicProtocolState {
    logout_enabled: bool,
    issuer: String,
    tokens: Arc<TokenService>,
}

/// Safe read-only endpoints. Hosted mutations are mounted separately.
pub fn router(issuer: &str, tokens: Arc<TokenService>) -> Router {
    router_with_logout(issuer, tokens, false)
}

pub fn router_with_logout(issuer: &str, tokens: Arc<TokenService>, logout_enabled: bool) -> Router {
    Router::new()
        .route("/.well-known/openid-configuration", get(discovery))
        .route("/oauth/jwks", get(jwks))
        .with_state(PublicProtocolState {
            logout_enabled,
            issuer: issuer.to_owned(),
            tokens,
        })
        .layer(cors::metadata())
}

async fn discovery(State(state): State<PublicProtocolState>) -> Json<impl serde::Serialize> {
    let mut metadata = provider_metadata(&state.issuer);
    if state.logout_enabled {
        metadata.end_session_endpoint = Some(format!(
            "{}/oauth/end-session",
            state.issuer.trim_end_matches('/')
        ));
    }
    Json(metadata)
}

async fn jwks(State(state): State<PublicProtocolState>) -> Json<JsonWebKeySet> {
    Json(state.tokens.jwks())
}
