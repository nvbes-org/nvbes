//! In-process token activity check for resource domains of this runtime.
//!
//! It asks the Identity domain the same question as RFC 7662 introspection,
//! without the HTTP round trip or a resource-server secret, and compares the
//! answer with the locally verified claims exactly like the remote client.

use std::{sync::Arc, time::Duration};

use nvbes_identity_sdk::introspection::{
    ActivityVerifier, ExpectedToken, IntrospectionError, confirm_activity,
};
use nvbes_identity_service::{
    oauth::clients::ClientRegistry, tokens::TokenService, tokens_claims::AccessTokenClaims,
};
use sqlx::PgPool;

const ACTIVITY_DEADLINE: Duration = Duration::from_secs(2);

pub struct InProcessActivity {
    db: PgPool,
    clients: Arc<ClientRegistry>,
    tokens: Arc<TokenService>,
}

impl InProcessActivity {
    pub fn new(db: PgPool, clients: Arc<ClientRegistry>, tokens: Arc<TokenService>) -> Self {
        Self {
            db,
            clients,
            tokens,
        }
    }
}

#[async_trait::async_trait]
impl ActivityVerifier for InProcessActivity {
    async fn is_active(
        &self,
        token: &str,
        expected: &ExpectedToken,
    ) -> Result<bool, IntrospectionError> {
        let claims = tokio::time::timeout(
            ACTIVITY_DEADLINE,
            self.tokens
                .introspect(&self.db, &self.clients, token, &expected.aud),
        )
        .await
        .map_err(|_| IntrospectionError::Unavailable)?
        .map_err(|_| IntrospectionError::Unavailable)?;
        match claims {
            None => Ok(false),
            Some(claims) => confirm_activity(&presented(claims)?, expected),
        }
    }
}

fn presented(claims: AccessTokenClaims) -> Result<ExpectedToken, IntrospectionError> {
    let token_type = claims.presentation_type().to_owned();
    let cnf = claims
        .cnf
        .map(serde_json::to_value)
        .transpose()
        .map_err(|_| IntrospectionError::Unavailable)?;
    Ok(ExpectedToken {
        sub: claims.sub,
        sid: claims.sid,
        grant_id: claims.grant_id,
        jti: claims.jti,
        iss: claims.iss,
        aud: claims.aud,
        client_id: claims.client_id,
        scope: claims.scope,
        exp: claims.exp,
        iat: claims.iat,
        nbf: claims.nbf,
        token_type,
        cnf,
    })
}

#[cfg(test)]
#[path = "api.identity.activity.tests.rs"]
mod tests;
