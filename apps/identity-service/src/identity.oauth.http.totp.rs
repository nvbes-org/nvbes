use super::{AuthorizationState, OAuthError, ProtocolError};
use crate::{
    browser::SessionProof,
    totp::{self, TotpError},
};
use axum::{
    Extension, Json,
    extract::{State, rejection::JsonRejection},
    response::IntoResponse,
};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Start {}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Confirm {
    factor_id: uuid::Uuid,
    code: String,
}

// Serde-derived structs also accept positional JSON arrays. This HTTP contract
// accepts objects only, while retaining the derived duplicate/unknown-field checks.
pub(super) struct Object<T>(T);
impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Object<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor<T>(std::marker::PhantomData<T>);
        impl<'de, T: serde::Deserialize<'de>> serde::de::Visitor<'de> for Visitor<T> {
            type Value = Object<T>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a JSON object")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                map: A,
            ) -> Result<Self::Value, A::Error> {
                T::deserialize(serde::de::value::MapAccessDeserializer::new(map)).map(Object)
            }
        }
        deserializer.deserialize_map(Visitor(std::marker::PhantomData))
    }
}

fn failure(error: TotpError) -> ProtocolError {
    ProtocolError::OAuth(match error {
        TotpError::Crypto | TotpError::Database(_) => OAuthError::Unavailable,
        _ => OAuthError::InvalidRequest,
    })
}

async fn quota(state: &AuthorizationState, proof: &SessionProof) -> Result<(), ProtocolError> {
    let principal: Option<uuid::Uuid> = sqlx::query_scalar("SELECT s.principal_id FROM identity_sessions s JOIN identity_principals p ON p.id=s.principal_id WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND p.status='active'")
        .bind(crate::auth::hash_token(proof.token())).fetch_optional(&state.db).await
        .map_err(|_| ProtocolError::OAuth(OAuthError::Unavailable))?;
    let principal = principal.ok_or(ProtocolError::OAuth(OAuthError::LoginRequired))?;
    crate::oauth::limits::enforce(
        &state.db,
        &state.limiter,
        crate::rate_limits::Category::MfaAccount,
        &principal.to_string(),
    )
    .await
    .map_err(ProtocolError::OAuth)
}

pub(super) async fn start(
    State(state): State<AuthorizationState>,
    Extension(proof): Extension<SessionProof>,
    body: Result<Json<Object<Start>>, JsonRejection>,
) -> Result<impl IntoResponse, ProtocolError> {
    quota(&state, &proof).await?;
    let Json(Object(Start {})) =
        body.map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    Ok(Json(
        totp::start(&state.db, &state.mfa, proof.token())
            .await
            .map_err(failure)?,
    ))
}

pub(super) async fn confirm(
    State(state): State<AuthorizationState>,
    Extension(proof): Extension<SessionProof>,
    body: Result<Json<Object<Confirm>>, JsonRejection>,
) -> Result<impl IntoResponse, ProtocolError> {
    quota(&state, &proof).await?;
    let Json(Object(form)) = body.map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    let expires = totp::confirm(
        &state.db,
        &state.mfa,
        proof.token(),
        form.factor_id,
        &form.code,
    )
    .await
    .map_err(failure)?;
    Ok(Json(
        serde_json::json!({"enrolled":true,"step_up":true,"expires_at":expires}),
    ))
}
