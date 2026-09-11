use super::BillingPrincipal;
use crate::{config::BillingConfig, error::BillingError};
use jsonwebtoken::{Algorithm, Validation, decode, decode_header};
use nvbes_dpop::resource::{Credentials, ResourceVerifier, binding};
use nvbes_identity_sdk::introspection::{ExpectedToken, IntrospectionClient};
use nvbes_identity_sdk::pinned_keys::PinnedKeySet;
use serde::Deserialize;
use uuid::Uuid;

const AUDIENCE: &str = "nvbes-billing-service";

#[derive(Deserialize)]
struct Claims {
    sub: Uuid,
    iss: String,
    aud: String,
    token_type: String,
    scope: String,
    amr: Vec<String>,
    client_id: String,
    exp: u64,
    iat: u64,
    nbf: u64,
    auth_time: u64,
    step_up_time: Option<u64>,
    step_up_expires_at: Option<u64>,
    jti: Uuid,
    sid: Uuid,
    grant_id: Uuid,
    cnf: Option<serde_json::Value>,
}

#[derive(Clone)]
struct ConfiguredVerifier {
    keys: PinnedKeySet,
    issuer: String,
    introspection: IntrospectionClient,
    dpop: Option<ResourceVerifier>,
}

#[derive(Clone)]
pub struct TokenVerifier(Option<ConfiguredVerifier>);

impl TokenVerifier {
    pub fn new(config: &BillingConfig) -> anyhow::Result<Self> {
        match (
            &config.identity_public_key_pem,
            &config.identity_token_issuer,
            &config.identity_token_key_id,
            &config.identity_resource_client_id,
            &config.identity_resource_secret,
        ) {
            (None, None, None, None, None) if config.identity_verification_keys == "[]" => {
                Ok(Self(None))
            }
            (Some(pem), Some(issuer), Some(key_id), Some(client_id), Some(secret)) => {
                let url = reqwest::Url::parse(issuer)?;
                anyhow::ensure!(
                    url.host_str().is_some()
                        && matches!(url.scheme(), "http" | "https")
                        && url.username().is_empty()
                        && url.password().is_none()
                        && url.query().is_none()
                        && url.fragment().is_none(),
                    "invalid Identity issuer"
                );
                anyhow::ensure!(
                    url.scheme() == "https"
                        || matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")),
                    "Identity HTTP issuer must be loopback"
                );
                anyhow::ensure!(
                    !key_id.is_empty()
                        && key_id.len() <= 128
                        && key_id
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.')),
                    "invalid Identity key identifier"
                );
                Ok(Self(Some(ConfiguredVerifier {
                    dpop: config
                        .public_origin
                        .as_deref()
                        .map(ResourceVerifier::new)
                        .transpose()?,
                    keys: PinnedKeySet::new(key_id, pem, &config.identity_verification_keys)?,
                    issuer: issuer.clone(),
                    introspection: IntrospectionClient::new(issuer, client_id, secret)?,
                })))
            }
            _ => anyhow::bail!(
                "Identity token verification and resource credentials must be configured together"
            ),
        }
    }

    pub async fn authenticate(
        &self,
        credentials: &Credentials<'_>,
        method: &axum::http::Method,
        uri: &axum::http::Uri,
        db: &sqlx::PgPool,
    ) -> Result<BillingPrincipal, BillingError> {
        let (principal, expected) = self.validated(credentials.token)?;
        let verifier = self.0.as_ref().ok_or(BillingError::Unauthorized)?;
        let proof = ResourceVerifier::verify(
            verifier.dpop.as_ref(),
            credentials,
            binding(expected.cnf.as_ref())?,
            method,
            uri,
        )?;
        match verifier
            .introspection
            .is_active(credentials.token, &expected)
            .await
        {
            Ok(true) => {}
            Ok(false) => return Err(BillingError::Unauthorized),
            Err(_) => return Err(BillingError::IdentityUnavailable),
        }
        if let Some(proof) = proof {
            proof.consume(db, expected.exp).await?;
        }
        Ok(principal)
    }

    #[cfg(test)]
    fn verify(&self, token: &str) -> Result<BillingPrincipal, BillingError> {
        self.validated(token).map(|(principal, _)| principal)
    }

    fn validated(&self, token: &str) -> Result<(BillingPrincipal, ExpectedToken), BillingError> {
        let verifier = self.0.as_ref().ok_or(BillingError::Unauthorized)?;
        if token.len() > 16_384 {
            return Err(BillingError::Unauthorized);
        }
        let header = decode_header(token).map_err(|_| BillingError::Unauthorized)?;
        if header.alg != Algorithm::RS256
            || header.typ.as_deref() != Some("at+jwt")
            || header.jku.is_some()
            || header.jwk.is_some()
            || header.x5u.is_some()
        {
            return Err(BillingError::Unauthorized);
        }
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&verifier.issuer]);
        validation.set_audience(&[AUDIENCE]);
        validation.set_required_spec_claims(&["iss", "aud", "sub", "exp", "iat", "nbf"]);
        validation.validate_nbf = true;
        validation.leeway = 0;
        let now = chrono::Utc::now().timestamp() as u64;
        let key = verifier
            .keys
            .key(header.kid.as_deref(), now)
            .map_err(|_| BillingError::Unauthorized)?;
        let claims = decode::<Claims>(token, key, &validation)
            .map_err(|_| BillingError::Unauthorized)?
            .claims;
        if claims.iss != verifier.issuer
            || claims.aud != AUDIENCE
            || claims.token_type != "access"
            || claims.client_id.is_empty()
            || claims.iat > now
            || claims.nbf != claims.iat
            || claims.exp <= now
            || claims.exp <= claims.iat
            || claims.exp - claims.iat > 900
            || claims.auth_time > claims.iat
            || [claims.sub, claims.jti, claims.sid, claims.grant_id]
                .iter()
                .any(Uuid::is_nil)
        {
            return Err(BillingError::Unauthorized);
        }
        if claims.amr.is_empty()
            || claims
                .amr
                .iter()
                .any(|m| !matches!(m.as_str(), "pwd" | "totp" | "webauthn"))
        {
            return Err(BillingError::Unauthorized);
        }
        let fresh_step_up = match (claims.step_up_time, claims.step_up_expires_at) {
            (None, None) => false,
            (Some(at), Some(until))
                if at >= claims.auth_time
                    && at <= claims.iat
                    && until > claims.iat
                    && until - at <= 600 =>
            {
                until > now
            }
            _ => return Err(BillingError::Unauthorized),
        };
        let scopes: Vec<String> = claims.scope.split(' ').map(str::to_owned).collect();
        if scopes
            .iter()
            .any(|s| !matches!(s.as_str(), "billing:read" | "billing:checkout"))
        {
            return Err(BillingError::Unauthorized);
        }
        let strong = claims
            .amr
            .iter()
            .any(|m| matches!(m.as_str(), "totp" | "webauthn"));
        let primary_passkey =
            claims.amr == ["webauthn"] && now.saturating_sub(claims.auth_time) <= 300;
        let expected = ExpectedToken {
            sub: claims.sub.to_string(),
            sid: claims.sid.to_string(),
            grant_id: claims.grant_id.to_string(),
            jti: claims.jti.to_string(),
            iss: claims.iss,
            aud: claims.aud,
            client_id: claims.client_id,
            scope: claims.scope,
            exp: claims.exp,
            iat: claims.iat,
            nbf: claims.nbf,
            token_type: if binding(claims.cnf.as_ref())
                .map_err(|_| BillingError::Unauthorized)?
                .is_some()
            {
                "DPoP"
            } else {
                "Bearer"
            }
            .into(),
            cnf: claims.cnf,
        };
        Ok((
            BillingPrincipal {
                id: claims.sub,
                scopes,
                strong_authentication: (fresh_step_up && strong) || primary_passkey,
            },
            expected,
        ))
    }
}

#[cfg(test)]
#[path = "billing.auth.tokens.tests.rs"]
mod tests;
