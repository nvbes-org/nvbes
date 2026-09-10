use super::TokenService;
use crate::{
    oauth::{
        clients::ClientRegistry,
        codes::{self, CodeExchange, ExchangeOutcome},
        dpop::verify_token_proof,
        error::OAuthError,
        store::StoreError,
    },
    tokens_claims::TokenSet,
    tokens_error::TokenError,
};
use sqlx::PgPool;

pub struct AuthorizationCodeRequest<'a> {
    pub code: &'a str,
    pub client_id: &'a str,
    pub redirect_uri: &'a str,
    pub verifier: &'a str,
    pub dpop_proof: Option<&'a str>,
}

impl TokenService {
    /// Exchange and signing share a commit; only raw cryptographically verified
    /// DPoP evidence is accepted at this boundary.
    pub async fn exchange_code(
        &self,
        db: &PgPool,
        clients: &ClientRegistry,
        input: AuthorizationCodeRequest<'_>,
    ) -> Result<TokenSet, TokenError> {
        let proof = input
            .dpop_proof
            .map(|proof| verify_token_proof(proof, "POST", &format!("{}oauth/token", self.issuer)))
            .transpose()?;
        let mut tx = db.begin().await?;
        let outcome = codes::exchange_in(
            &mut tx,
            clients,
            CodeExchange {
                code: input.code,
                client_id: input.client_id,
                redirect_uri: input.redirect_uri,
                verifier: input.verifier,
                verified_dpop_jkt: proof.as_ref().map(|proof| proof.thumbprint()),
            },
        )
        .await
        .map_err(|error| match error {
            StoreError::Protocol(error) => TokenError::Authorization(error),
            StoreError::Database(error) => TokenError::Database(error),
            StoreError::Serialization(_) => TokenError::Authorization(OAuthError::Unavailable),
        })?;
        if let Some(proof) = proof {
            proof.consume(&mut tx).await?;
        }
        let grant = match outcome {
            ExchangeOutcome::Granted(grant) => grant,
            ExchangeOutcome::Replayed => {
                tx.commit().await?;
                return Err(OAuthError::InvalidGrant.into());
            }
        };
        let response = self.issue_grant_in(&mut tx, clients, &grant).await?;
        tx.commit().await?;
        Ok(response)
    }
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.tokens.exchange.tests.rs"]
mod tests;
