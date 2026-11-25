use super::{TokenService, request_allows_refresh};
use crate::{
    oauth::{clients::ClientRegistry, dpop, error::OAuthError, store},
    tokens_claims::TokenSet,
    tokens_error::TokenError,
    tokens_grants,
};
use sqlx::PgPool;
use uuid::Uuid;

/// The proof is verified here; callers cannot substitute a declared thumbprint.
pub struct RefreshRequest<'a> {
    pub refresh_token: &'a str,
    pub client_id: &'a str,
    pub dpop_proof: Option<&'a str>,
}

impl TokenService {
    /// Verify client/key binding before rotating or revoking any refresh secret.
    pub async fn refresh(
        &self,
        db: &PgPool,
        clients: &ClientRegistry,
        request: RefreshRequest<'_>,
    ) -> Result<TokenSet, TokenError> {
        if request.refresh_token.is_empty() || request.refresh_token.len() > 4096 {
            return Err(TokenError::InactiveGrant);
        }
        clients.get(request.client_id)?;
        let proof = request
            .dpop_proof
            .map(|proof| dpop::verify_token_proof(proof, "POST", &self.endpoint("oauth/token")))
            .transpose()?;
        let hash = store::hash(request.refresh_token);
        let mut tx = db.begin().await?;
        // Lookup only: all rotations acquire the grant lock before refresh rows,
        // including concurrent replays of different generations in one family.
        let grant_id: Uuid = sqlx::query_scalar(
            "SELECT grant_id FROM identity_oauth_refresh_tokens WHERE token_hash=$1 AND client_id=$2",
        ).bind(&hash).bind(request.client_id)
            .fetch_optional(&mut *tx).await?.ok_or(TokenError::InactiveGrant)?;
        let active = tokens_grants::load(&mut tx, clients, grant_id).await?;
        if active.request.client_id() != request.client_id
            || active.token_issued_at.is_none()
            || !request_allows_refresh(&active.request)
        {
            return Err(TokenError::InactiveGrant);
        }
        if active.request.dpop_jkt.as_deref() != proof.as_ref().map(|p| p.thumbprint()) {
            return Err(OAuthError::InvalidDpopProof.into());
        }
        let (family_id, already_used): (Uuid, bool) = sqlx::query_as(
            "SELECT family_id,(rotated_at IS NOT NULL OR revoked_at IS NOT NULL) FROM identity_oauth_refresh_tokens WHERE token_hash=$1 AND grant_id=$2 AND client_id=$3 FOR UPDATE",
        ).bind(&hash).bind(grant_id).bind(request.client_id)
            .fetch_optional(&mut *tx).await?.ok_or(TokenError::InactiveGrant)?;
        if let Some(proof) = proof {
            proof.consume(&mut tx).await?;
        }
        if already_used {
            sqlx::query("UPDATE identity_oauth_refresh_tokens SET revoked_at=clock_timestamp() WHERE family_id=$1 AND revoked_at IS NULL")
                .bind(family_id).execute(&mut *tx).await?;
            sqlx::query("UPDATE identity_oauth_grants SET revoked_at=clock_timestamp() WHERE id=$1 AND revoked_at IS NULL")
                .bind(grant_id).execute(&mut *tx).await?;
            store::audit(
                &mut tx,
                active.principal_id,
                "identity.oauth.refresh_replayed",
            )
            .await?;
            tx.commit().await?;
            return Err(TokenError::InactiveGrant);
        }
        let response = self.sign_grant(&active)?;
        let next = store::random_secret();
        sqlx::query("UPDATE identity_oauth_refresh_tokens SET rotated_at=clock_timestamp() WHERE token_hash=$1")
            .bind(&hash).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO identity_oauth_refresh_tokens(token_hash,family_id,grant_id,principal_id,client_id) VALUES($1,$2,$3,$4,$5)")
            .bind(store::hash(&next)).bind(family_id).bind(grant_id)
            .bind(active.principal_id).bind(request.client_id).execute(&mut *tx).await?;
        store::audit(
            &mut tx,
            active.principal_id,
            "identity.oauth.refresh_rotated",
        )
        .await?;
        tx.commit().await?;
        Ok(TokenSet {
            refresh_token: Some(next),
            ..response
        })
    }
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.tokens.refresh.tests.rs"]
mod tests;
