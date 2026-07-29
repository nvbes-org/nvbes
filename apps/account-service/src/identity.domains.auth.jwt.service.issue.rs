use chrono::Utc;
use uuid::Uuid;

use super::types::{
    IdTokenClaims, JwtService, LogoutTokenClaims, SecurityEventTokenClaims, TokenClaims,
    TokenConfirmation, TokenPair,
};
use crate::http::error::AppError;

const CLOUD_TOKEN_AUDIENCE: &str = "nvbes-cloud-service";

impl JwtService {
    pub async fn generate_logout_token(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        audience: &str,
    ) -> Result<String, AppError> {
        let now = Utc::now();
        self.encode_typed_claims(
            &LogoutTokenClaims {
                iss: self.issuer.clone(),
                sub: user_id.to_string(),
                aud: audience.to_string(),
                iat: now.timestamp(),
                exp: (now + chrono::Duration::minutes(2)).timestamp(),
                jti: Uuid::new_v4().to_string(),
                sid: session_id.to_string(),
                events: serde_json::json!({
                    "http://schemas.openid.net/event/backchannel-logout": {}
                }),
            },
            Some("logout+jwt"),
        )
        .await
    }

    pub async fn generate_caep_session_revoked_token(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        audience: &str,
    ) -> Result<String, AppError> {
        let now = Utc::now();
        self.encode_typed_claims(
            &SecurityEventTokenClaims {
                iss: self.issuer.clone(),
                aud: audience.to_string(),
                iat: now.timestamp(),
                jti: Uuid::new_v4().to_string(),
                sub_id: Some(serde_json::json!({
                    "format": "complex",
                    "user": {
                        "format": "iss_sub",
                        "iss": self.issuer.clone(),
                        "sub": user_id,
                    },
                    "session": {
                        "format": "opaque",
                        "id": session_id,
                    }
                })),
                events: serde_json::json!({
                    "https://schemas.openid.net/secevent/caep/event-type/session-revoked": {
                        "event_timestamp": now.timestamp(),
                    }
                }),
            },
            Some("secevent+jwt"),
        )
        .await
    }

    pub async fn generate_risc_credential_compromise_token(
        &self,
        user_id: Uuid,
        audience: &str,
    ) -> Result<String, AppError> {
        let now = Utc::now();
        self.encode_typed_claims(
            &SecurityEventTokenClaims {
                iss: self.issuer.clone(),
                aud: audience.to_string(),
                iat: now.timestamp(),
                jti: Uuid::new_v4().to_string(),
                sub_id: None,
                events: serde_json::json!({
                    "https://schemas.openid.net/secevent/risc/event-type/credential-compromise": {
                        "subject": {
                            "format": "iss_sub",
                            "iss": self.issuer.clone(),
                            "sub": user_id,
                        },
                        "credential_type": "refresh_token",
                        "event_timestamp": now.timestamp(),
                    }
                }),
            },
            Some("secevent+jwt"),
        )
        .await
    }

    pub async fn generate_id_token(
        &self,
        user_id: Uuid,
        client_id: &str,
        nonce: &str,
        auth_time: i64,
    ) -> Result<String, AppError> {
        let now = Utc::now();
        self.encode_claims(&IdTokenClaims {
            iss: self.issuer.clone(),
            sub: user_id.to_string(),
            aud: client_id.to_string(),
            exp: (now + chrono::Duration::minutes(5)).timestamp(),
            iat: now.timestamp(),
            auth_time,
            nonce: nonce.to_string(),
        })
        .await
    }

    #[cfg(test)]
    pub async fn generate_token_pair(
        &self,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        scope: &str,
    ) -> Result<TokenPair, AppError> {
        self.generate_token_pair_with_session(
            user_id,
            workspace_id,
            None,
            scope,
            None,
            None,
            None,
            Some("aal1"),
            Some(vec!["pwd".to_string()]),
            None,
            None,
            None,
        )
        .await
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Token issuance keeps claims sources explicit across flows."
    )]
    pub async fn generate_token_pair_with_session(
        &self,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        workspace_region: Option<String>,
        scope: &str,
        session_id: Option<Uuid>,
        tenant_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        acr: Option<&str>,
        amr: Option<Vec<String>>,
        client_id: Option<&str>,
        auth_time: Option<i64>,
        cnf_jkt: Option<String>,
    ) -> Result<TokenPair, AppError> {
        self.generate_token_pair_with_authorization_details(
            user_id,
            workspace_id,
            workspace_region,
            scope,
            Vec::new(),
            session_id,
            tenant_id,
            organization_id,
            acr,
            amr,
            client_id,
            auth_time,
            cnf_jkt,
        )
        .await
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Authorization details issuance keeps all claim inputs explicit."
    )]
    pub async fn generate_token_pair_with_authorization_details(
        &self,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        workspace_region: Option<String>,
        scope: &str,
        authorization_details: Vec<serde_json::Value>,
        session_id: Option<Uuid>,
        tenant_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        acr: Option<&str>,
        amr: Option<Vec<String>>,
        client_id: Option<&str>,
        auth_time: Option<i64>,
        cnf_jkt: Option<String>,
    ) -> Result<TokenPair, AppError> {
        self.generate_token_pair_with_confirmation(
            user_id,
            workspace_id,
            workspace_region,
            scope,
            authorization_details,
            session_id,
            tenant_id,
            organization_id,
            acr,
            amr,
            client_id,
            auth_time,
            cnf_jkt.map(TokenConfirmation::dpop),
        )
        .await
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Authorization details issuance keeps all claim inputs explicit."
    )]
    pub async fn generate_token_pair_with_confirmation(
        &self,
        user_id: Uuid,
        workspace_id: Option<Uuid>,
        workspace_region: Option<String>,
        scope: &str,
        authorization_details: Vec<serde_json::Value>,
        session_id: Option<Uuid>,
        tenant_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        acr: Option<&str>,
        amr: Option<Vec<String>>,
        client_id: Option<&str>,
        auth_time: Option<i64>,
        confirmation: Option<TokenConfirmation>,
    ) -> Result<TokenPair, AppError> {
        let now = Utc::now();
        let session_id = session_id.unwrap_or_else(Uuid::new_v4);
        let access_jti = Uuid::new_v4().to_string();
        let refresh_jti = Uuid::new_v4().to_string();
        let auth_time = auth_time.unwrap_or_else(|| now.timestamp());
        let amr = amr.unwrap_or_else(|| vec!["pwd".to_string()]);
        let access_claims = TokenClaims {
            jti: access_jti.clone(),
            sid: session_id.to_string(),
            sub: user_id.to_string(),
            workspace_id: workspace_id.map(|id| id.to_string()),
            workspace_region: workspace_region.clone(),
            tenant_id: tenant_id.map(|id| id.to_string()),
            organization_id: organization_id.map(|id| id.to_string()),
            token_type: "access".to_string(),
            scope: scope.to_string(),
            authorization_details: authorization_details.clone(),
            acr: acr.map(|v| v.to_string()),
            amr: amr.clone(),
            client_id: client_id.map(|v| v.to_string()),
            auth_time: Some(auth_time),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            exp: (now + self.access_token_expiry).timestamp(),
            cnf: confirmation.clone(),
            act: None,
        };
        let access_token = self.encode_token(&access_claims).await?;
        let refresh_claims = TokenClaims {
            jti: refresh_jti.clone(),
            sid: session_id.to_string(),
            sub: user_id.to_string(),
            workspace_id: workspace_id.map(|id| id.to_string()),
            workspace_region,
            tenant_id: tenant_id.map(|id| id.to_string()),
            organization_id: organization_id.map(|id| id.to_string()),
            token_type: "refresh".to_string(),
            scope: "refresh".to_string(),
            authorization_details,
            acr: acr.map(|v| v.to_string()),
            amr,
            client_id: client_id.map(|v| v.to_string()),
            auth_time: Some(auth_time),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            exp: (now + self.refresh_token_expiry).timestamp(),
            cnf: confirmation.clone(),
            act: None,
        };
        let refresh_token = self.encode_token(&refresh_claims).await?;
        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: if confirmation.as_ref().is_some_and(|cnf| cnf.jkt.is_some()) {
                "DPoP".to_string()
            } else {
                "Bearer".to_string()
            },
            expires_in: self.access_token_expiry.num_seconds(),
            refresh_jti,
            session_id,
            cnf_jkt: confirmation.as_ref().and_then(|cnf| cnf.jkt.clone()),
            cnf_x5t_s256: confirmation.as_ref().and_then(|cnf| cnf.x5t_s256.clone()),
        })
    }

    pub async fn generate_access_token_from_claims(
        &self,
        claims: &TokenClaims,
    ) -> Result<String, AppError> {
        let now = Utc::now();
        let new_claims = TokenClaims {
            jti: Uuid::new_v4().to_string(),
            exp: (now + self.access_token_expiry).timestamp(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            cnf: claims.cnf.clone(),
            ..claims.clone()
        };
        self.encode_token(&new_claims).await
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "M2M issuance keeps subject, workspace, and audience explicit."
    )]
    pub async fn generate_m2m_access_token(
        &self,
        client_id: &str,
        principal_id: Uuid,
        tenant_id: Uuid,
        organization_id: Option<Uuid>,
        workspace_id: Uuid,
        workspace_region: Option<String>,
        scope: &str,
        audience: Option<&str>,
    ) -> Result<String, AppError> {
        self.generate_m2m_access_token_bound(
            client_id,
            principal_id,
            tenant_id,
            organization_id,
            workspace_id,
            workspace_region,
            scope,
            audience,
            None,
        )
        .await
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "M2M issuance keeps subject, workspace, audience, and sender binding explicit."
    )]
    pub async fn generate_m2m_access_token_bound(
        &self,
        client_id: &str,
        principal_id: Uuid,
        tenant_id: Uuid,
        organization_id: Option<Uuid>,
        workspace_id: Uuid,
        workspace_region: Option<String>,
        scope: &str,
        audience: Option<&str>,
        confirmation: Option<TokenConfirmation>,
    ) -> Result<String, AppError> {
        let now = Utc::now();
        let claims = TokenClaims {
            jti: Uuid::new_v4().to_string(),
            sid: Uuid::nil().to_string(),
            sub: principal_id.to_string(),
            workspace_id: Some(workspace_id.to_string()),
            workspace_region,
            tenant_id: Some(tenant_id.to_string()),
            organization_id: organization_id.map(|id| id.to_string()),
            token_type: "access".to_string(),
            scope: scope.to_string(),
            authorization_details: Vec::new(),
            acr: None,
            amr: vec!["m2m".to_string()],
            client_id: Some(client_id.to_string()),
            auth_time: Some(now.timestamp()),
            iss: self.issuer.clone(),
            aud: audience.unwrap_or(CLOUD_TOKEN_AUDIENCE).to_string(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            exp: (now + self.access_token_expiry).timestamp(),
            cnf: confirmation,
            act: None,
        };
        self.encode_token(&claims).await
    }

    pub async fn generate_token_exchange(
        &self,
        subject_claims: &TokenClaims,
        actor_sub: &str,
        actor_client_id: Option<&str>,
        scope: &str,
        audience: Option<&str>,
        confirmation: Option<TokenConfirmation>,
    ) -> Result<String, AppError> {
        let now = Utc::now();
        let claims = TokenClaims {
            jti: Uuid::new_v4().to_string(),
            sid: subject_claims.sid.clone(),
            sub: subject_claims.sub.clone(),
            workspace_id: subject_claims.workspace_id.clone(),
            workspace_region: subject_claims.workspace_region.clone(),
            tenant_id: subject_claims.tenant_id.clone(),
            organization_id: subject_claims.organization_id.clone(),
            token_type: "access".to_string(),
            scope: scope.to_string(),
            authorization_details: subject_claims.authorization_details.clone(),
            acr: subject_claims.acr.clone(),
            amr: subject_claims.amr.clone(),
            client_id: subject_claims.client_id.clone(),
            auth_time: subject_claims.auth_time,
            iss: self.issuer.clone(),
            aud: audience.unwrap_or(CLOUD_TOKEN_AUDIENCE).to_string(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            exp: (now + self.access_token_expiry).timestamp(),
            cnf: confirmation,
            act: Some(super::types::ActorClaim {
                sub: actor_sub.to_string(),
                client_id: actor_client_id.map(|v| v.to_string()),
            }),
        };
        self.encode_token(&claims).await
    }
}
