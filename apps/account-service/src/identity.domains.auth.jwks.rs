use axum::Json;
use axum::extract::State;
use serde::Serialize;
use utoipa::ToSchema;

use super::keys::{JwkEntry, get_keys_for_jwks};
use crate::app::AppState;
use crate::http::error::AppError;
use nvbes_core::http::error::ErrorEnvelope;

#[derive(Debug, Serialize, ToSchema)]
pub struct JwksResponse {
    pub keys: Vec<JwkKey>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct JwkKey {
    pub kty: String,
    pub kid: String,
    pub alg: String,
    #[serde(rename = "use")]
    pub use_: String,
    pub n: String,
    pub e: String,
}

impl From<JwkEntry> for JwkKey {
    fn from(entry: JwkEntry) -> Self {
        Self {
            kty: "RSA".to_string(),
            kid: entry.kid,
            alg: "RS256".to_string(),
            use_: "sig".to_string(),
            n: entry.n,
            e: entry.e,
        }
    }
}

#[utoipa::path(
    get,
    path = "/.well-known/jwks.json",
    tag = "auth",
    responses(
        (status = 200, description = "JWKS public keys", body = JwksResponse),
        (status = 500, description = "Internal server error", body = ErrorEnvelope),
    ),
)]
pub async fn get_jwks(State(state): State<AppState>) -> Result<Json<JwksResponse>, AppError> {
    let entries = get_keys_for_jwks(&state.db).await?;
    let keys = entries.into_iter().map(JwkKey::from).collect();
    Ok(Json(JwksResponse { keys }))
}
