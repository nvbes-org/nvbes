use super::TokenService;
use crate::{
    oauth::{clients::ClientRegistry, dpop::verify_resource_proof, error::OAuthError},
    tokens_error::TokenError,
    tokens_grants, tokens_policy,
};
use sqlx::PgPool;

impl TokenService {
    pub(crate) async fn userinfo(
        &self,
        db: &PgPool,
        clients: &ClientRegistry,
        token: &str,
        dpop_scheme: bool,
        proof: Option<&str>,
        method: &str,
    ) -> Result<serde_json::Value, TokenError> {
        let claims = self.verify(token, tokens_policy::USERINFO_AUDIENCE)?;
        let proof = match (&claims.cnf, dpop_scheme, proof) {
            (Some(binding), true, Some(proof)) => {
                let verified =
                    verify_resource_proof(proof, method, &self.endpoint("oauth/userinfo"), token)?;
                if verified.thumbprint() != binding.jkt {
                    return Err(OAuthError::InvalidDpopProof.into());
                }
                Some(verified)
            }
            (None, false, None) => None,
            (Some(_), true, None) => return Err(OAuthError::InvalidDpopProof.into()),
            _ => return Err(TokenError::InvalidToken),
        };
        let mut tx = db.begin().await?;
        let id = uuid::Uuid::parse_str(&claims.grant_id).map_err(|_| TokenError::InvalidToken)?;
        let grant = tokens_grants::load(&mut tx, clients, id).await?;
        if grant.token_issued_at.is_none()
            || grant.session_id.to_string() != claims.sid
            || grant.principal_id.to_string() != claims.sub
            || grant.request.client_id() != claims.client_id
            || grant.request.audience() != claims.aud
            || grant.request.resource() != self.endpoint("oauth/userinfo")
            || grant.request.dpop_jkt.as_deref() != claims.cnf.as_ref().map(|c| c.jkt.as_str())
            || tokens_policy::access_scope(grant.request.audience(), grant.request.scope())?
                != claims.scope
        {
            return Err(TokenError::InvalidToken);
        }
        if let Some(proof) = proof {
            proof.consume(&mut tx).await?;
        }
        let mut result = serde_json::json!({"sub":claims.sub});
        if claims.scope.split(' ').any(|scope| scope == "email") {
            let email:Option<String> = sqlx::query_scalar("SELECT normalized_value FROM identity_login_identifiers WHERE principal_id=$1 AND kind='email' AND verified_at IS NOT NULL ORDER BY created_at,id LIMIT 1")
                .bind(grant.principal_id).fetch_optional(&mut *tx).await?;
            if let Some(email) = email {
                result["email"] = email.into();
                result["email_verified"] = true.into();
            }
        }
        tx.commit().await?;
        Ok(result)
    }
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.tokens.userinfo.tests.rs"]
mod tests;
