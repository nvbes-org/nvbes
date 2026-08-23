use chrono::Duration;
use jsonwebtoken::EncodingKey;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenConfirmation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jkt: Option<String>,
    #[serde(rename = "x5t#S256", skip_serializing_if = "Option::is_none")]
    pub x5t_s256: Option<String>,
}

impl TokenConfirmation {
    pub fn dpop(jkt: String) -> Self {
        Self {
            jkt: Some(jkt),
            x5t_s256: None,
        }
    }

    pub fn mtls(thumbprint: String) -> Self {
        Self {
            jkt: None,
            x5t_s256: Some(thumbprint),
        }
    }
}

pub struct TokenPairIssueRequest<'a> {
    pub user_id: Uuid,
    pub access_token_audience: &'a str,
    pub workspace_id: Option<Uuid>,
    pub workspace_region: Option<String>,
    pub scope: &'a str,
    pub authorization_details: Vec<serde_json::Value>,
    pub session_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub acr: Option<&'a str>,
    pub amr: Option<Vec<String>>,
    pub client_id: Option<&'a str>,
    pub auth_time: Option<i64>,
    pub confirmation: Option<TokenConfirmation>,
}

impl<'a> TokenPairIssueRequest<'a> {
    pub fn new(user_id: Uuid, access_token_audience: &'a str, scope: &'a str) -> Self {
        Self {
            user_id,
            access_token_audience,
            workspace_id: None,
            workspace_region: None,
            scope,
            authorization_details: Vec::new(),
            session_id: None,
            tenant_id: None,
            organization_id: None,
            acr: None,
            amr: None,
            client_id: None,
            auth_time: None,
            confirmation: None,
        }
    }
}

pub struct M2mAccessTokenIssueRequest<'a> {
    pub client_id: &'a str,
    pub principal_id: Uuid,
    pub tenant_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Uuid,
    pub workspace_region: Option<String>,
    pub scope: &'a str,
    pub audience: &'a str,
    pub confirmation: Option<TokenConfirmation>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActorClaim {
    pub sub: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenClaims {
    pub jti: String,
    pub sid: String,
    pub sub: String,
    pub workspace_id: Option<String>,
    pub workspace_region: Option<String>,
    pub tenant_id: Option<String>,
    pub organization_id: Option<String>,
    pub token_type: String,
    pub scope: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub authorization_details: Vec<serde_json::Value>,
    pub acr: Option<String>,
    pub amr: Vec<String>,
    pub client_id: Option<String>,
    pub auth_time: Option<i64>,
    pub iss: String,
    pub aud: String,
    pub exp: i64,
    pub iat: i64,
    pub nbf: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnf: Option<TokenConfirmation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act: Option<ActorClaim>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IdTokenClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub exp: i64,
    pub iat: i64,
    pub auth_time: i64,
    pub nonce: String,
}

#[derive(Debug, Serialize)]
pub struct LogoutTokenClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
    pub sid: String,
    pub events: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct SecurityEventTokenClaims {
    pub iss: String,
    pub aud: String,
    pub iat: i64,
    pub jti: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_id: Option<serde_json::Value>,
    pub events: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_jti: String,
    #[serde(skip_serializing)]
    pub session_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnf_jkt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnf_x5t_s256: Option<String>,
}

#[derive(Clone)]
pub(crate) enum Signer {
    Local {
        encoding_key: EncodingKey,
    },
    Kms {
        signer: Arc<super::kms::ScalewayKmsSigner>,
    },
}

#[derive(Clone)]
pub struct JwtService {
    pub(crate) signer: Signer,
    pub(crate) kid: String,
    pub(crate) decoding_keys: Vec<(String, jsonwebtoken::DecodingKey)>,
    pub(crate) issuer: String,
    pub(crate) audience: String,
    pub(crate) access_token_expiry: Duration,
    pub(crate) refresh_token_expiry: Duration,
}
