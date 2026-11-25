use super::Principal;
use crate::error::AccountError;
use jsonwebtoken::{Algorithm, Validation, decode, decode_header};
use nvbes_dpop::resource::{Credentials, ResourceVerifier, binding};
use nvbes_identity_sdk::introspection::{ActivityVerifier, ExpectedToken};
use nvbes_identity_sdk::pinned_keys::PinnedKeySet;
use serde::Deserialize;
use std::{collections::BTreeSet, sync::Arc};
use uuid::Uuid;

pub const ACCOUNT_AUDIENCE: &str = "nvbes-account-service";

/// Signing trust published by the Identity issuer of this runtime.
#[derive(Clone, PartialEq, Eq)]
pub struct TokenTrust {
    pub issuer: String,
    pub key_id: String,
    pub public_key_pem: String,
    /// Overlap keys as a JSON array of `{kid, public_key_pem, accept_until}`.
    pub verification_keys: String,
}

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
    keys: PinnedKeySet,
    issuer: String,
    activity: Arc<dyn ActivityVerifier>,
    dpop: Option<ResourceVerifier>,
}

impl TokenVerifier {
    pub fn new(
        trust: &TokenTrust,
        public_origin: Option<&str>,
        activity: Arc<dyn ActivityVerifier>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            dpop: public_origin.map(ResourceVerifier::new).transpose()?,
            keys: PinnedKeySet::new(
                &trust.key_id,
                &trust.public_key_pem,
                &trust.verification_keys,
            )?,
            issuer: trust.issuer.clone(),
            activity,
        })
    }

    pub(super) async fn authenticate(
        &self,
        credentials: &Credentials<'_>,
        method: &axum::http::Method,
        uri: &axum::http::Uri,
        db: &sqlx::PgPool,
    ) -> Result<Principal, AccountError> {
        let (principal, expected) = self.validated(credentials.token)?;
        let proof = ResourceVerifier::verify(
            self.dpop.as_ref(),
            credentials,
            binding(expected.cnf.as_ref())?,
            method,
            uri,
        )?;
        match self.activity.is_active(credentials.token, &expected).await {
            Ok(true) => {}
            Ok(false) => return Err(AccountError::Unauthorized),
            Err(_) => return Err(AccountError::IdentityUnavailable),
        }
        if let Some(proof) = proof {
            proof.consume(db, expected.exp).await?;
        }
        Ok(principal)
    }

    fn validated(&self, token: &str) -> Result<(Principal, ExpectedToken), AccountError> {
        let invalid = || AccountError::Unauthorized;
        if token.len() > 16_384 {
            return Err(invalid());
        }
        let header = decode_header(token).map_err(|_| invalid())?;
        if header.alg != Algorithm::RS256
            || header.typ.as_deref() != Some("at+jwt")
            || header.jku.is_some()
            || header.jwk.is_some()
            || header.x5u.is_some()
        {
            return Err(invalid());
        }
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[ACCOUNT_AUDIENCE]);
        validation.set_required_spec_claims(&["exp", "iat", "nbf", "iss", "aud", "sub"]);
        validation.validate_nbf = true;
        validation.leeway = 0;
        let now = chrono::Utc::now().timestamp() as u64;
        let key = self
            .keys
            .key(header.kid.as_deref(), now)
            .map_err(|_| invalid())?;
        let claims = decode::<AccessTokenClaims>(token, key, &validation)
            .map_err(|_| invalid())?
            .claims;
        let mut expected = claims.token;
        if expected.iss != self.issuer
            || expected.aud != ACCOUNT_AUDIENCE
            || expected.token_type != "access"
            || expected.client_id.is_empty()
            || expected.iat > now
            || expected.nbf != expected.iat
            || expected.exp <= now
            || expected.exp <= expected.iat
            || expected.exp - expected.iat > 900
            || claims.auth_time > expected.iat
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
        expected.token_type = if binding(expected.cnf.as_ref())
            .map_err(|_| invalid())?
            .is_some()
        {
            "DPoP"
        } else {
            "Bearer"
        }
        .into();
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
