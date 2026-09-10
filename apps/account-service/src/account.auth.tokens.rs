use super::Principal;
use crate::{config::AccountConfig, error::AccountError};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use nvbes_identity_sdk::introspection::{ExpectedToken, IntrospectionClient};
use serde::Deserialize;
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Deserialize)]
struct AccessTokenClaims {
    #[serde(flatten)]
    token: ExpectedToken,
    amr: Vec<String>,
    auth_time: u64,
    step_up_time: Option<u64>,
    step_up_expires_at: Option<u64>,
}

pub struct TokenVerifier {
    key: DecodingKey,
    issuer: String,
    key_id: String,
    introspection: IntrospectionClient,
}

impl TokenVerifier {
    pub fn new(config: &AccountConfig) -> anyhow::Result<Self> {
        anyhow::ensure!(
            config.token_audience == "nvbes-account-service",
            "invalid Account token audience"
        );
        Ok(Self {
            key: DecodingKey::from_rsa_pem(config.token_public_key_pem.as_bytes())?,
            issuer: config.token_issuer.clone(),
            key_id: config.token_key_id.clone(),
            introspection: IntrospectionClient::new(
                &config.token_issuer,
                &config.identity_resource_client_id,
                &config.identity_resource_secret,
            )?,
        })
    }

    pub(super) async fn authenticate(&self, token: &str) -> Result<Principal, AccountError> {
        let (principal, expected) = self.validated(token)?;
        match self.introspection.is_active(token, &expected).await {
            Ok(true) => Ok(principal),
            Ok(false) => Err(AccountError::Unauthorized),
            Err(_) => Err(AccountError::IdentityUnavailable),
        }
    }

    fn validated(&self, token: &str) -> Result<(Principal, ExpectedToken), AccountError> {
        let invalid = || AccountError::Unauthorized;
        if token.len() > 16_384 {
            return Err(invalid());
        }
        let header = decode_header(token).map_err(|_| invalid())?;
        if header.alg != Algorithm::RS256
            || header.typ.as_deref() != Some("at+jwt")
            || header.kid.as_deref() != Some(&self.key_id)
            || header.jku.is_some()
            || header.jwk.is_some()
            || header.x5u.is_some()
        {
            return Err(invalid());
        }
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&["nvbes-account-service"]);
        validation.set_required_spec_claims(&["exp", "iat", "nbf", "iss", "aud", "sub"]);
        validation.validate_nbf = true;
        validation.leeway = 0;
        let claims = decode::<AccessTokenClaims>(token, &self.key, &validation)
            .map_err(|_| invalid())?
            .claims;
        let now = chrono::Utc::now().timestamp() as u64;
        let mut expected = claims.token;
        if expected.iss != self.issuer
            || expected.aud != "nvbes-account-service"
            || expected.token_type != "access"
            || expected.client_id.is_empty()
            || expected.iat > now
            || expected.nbf != expected.iat
            || expected.exp <= now
            || expected.exp <= expected.iat
            || expected.exp - expected.iat > 900
            || claims.auth_time > expected.iat
            || expected.cnf.is_some()
        {
            return Err(invalid());
        }
        for id in [
            &expected.sub,
            &expected.sid,
            &expected.grant_id,
            &expected.jti,
        ] {
            if Uuid::parse_str(id).map_err(|_| invalid())?.is_nil() {
                return Err(invalid());
            }
        }
        let id = Uuid::parse_str(&expected.sub).map_err(|_| invalid())?;
        let scopes: Vec<String> = expected.scope.split(' ').map(str::to_owned).collect();
        if scopes.iter().any(|s| {
            !matches!(
                s.as_str(),
                "account:read" | "account:write" | "account:export" | "account:close"
            )
        }) || scopes.iter().collect::<BTreeSet<_>>().len() != scopes.len()
        {
            return Err(invalid());
        }
        if !claims
            .amr
            .iter()
            .any(|m| matches!(m.as_str(), "pwd" | "webauthn"))
            || claims
                .amr
                .iter()
                .any(|m| !matches!(m.as_str(), "pwd" | "webauthn" | "totp"))
            || claims.amr.iter().collect::<BTreeSet<_>>().len() != claims.amr.len()
        {
            return Err(invalid());
        }
        let step_up = match (claims.step_up_time, claims.step_up_expires_at) {
            (None, None) => None,
            (Some(at), Some(until))
                if at >= claims.auth_time
                    && at <= expected.iat
                    && until > expected.iat
                    && until - at <= 600
                    && claims
                        .amr
                        .iter()
                        .any(|m| matches!(m.as_str(), "totp" | "webauthn")) =>
            {
                Some(until)
            }
            _ => return Err(invalid()),
        };
        let primary = if claims.amr == ["webauthn"] {
            Some(claims.auth_time.saturating_add(300))
        } else {
            None
        };
        let strong_until = step_up
            .into_iter()
            .chain(primary)
            .max()
            .map(|until| until.min(expected.exp));
        expected.token_type = "Bearer".into();
        Ok((
            Principal {
                id,
                scopes,
                strong_until,
            },
            expected,
        ))
    }
}

#[cfg(test)]
#[path = "account.auth.tokens.tests.rs"]
mod tests;
