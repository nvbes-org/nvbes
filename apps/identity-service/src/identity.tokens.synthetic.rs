use crate::{auth, tokens::TokenService};
use nvbes_identity_service::oauth::{
    clients::ClientRegistry,
    codes::{self, CodeExchange},
    request::AuthorizationInput,
    store::{self, RequestKind},
};
use nvbes_identity_service::tokens::ACCESS_TOKEN_TTL_SECONDS;
use nvbes_identity_service::tokens_claims::AccessTokenClaims;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

fn clients(audience: &str) -> anyhow::Result<ClientRegistry> {
    anyhow::ensure!(
        audience == "nvbes-account-service",
        "synthetic token proof requires Account audience"
    );
    Ok(ClientRegistry::from_json(&serde_json::json!([{
        "client_id":"synthetic-account", "display_name":"Synthetic Account",
        "redirect_uris":["https://synthetic.example.invalid/callback"],
        "post_logout_redirect_uris":[],
        "resources":{"https://synthetic.example.invalid/api":{
            "audience":audience,"scopes":["account:read","account:write","account:export","account:close"]}},
        "allow_refresh":false,"require_dpop":false
    }]).to_string(), false)?)
}

/// Operator-only diagnostics exercise the real grant path, with no signing bypass.
pub(crate) async fn issue_for_session(
    service: &TokenService,
    db: &PgPool,
    session_token: &str,
    audience: &str,
    scope: &str,
) -> anyhow::Result<String> {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use sha2::{Digest, Sha256};
    let clients = clients(audience)?;
    let verifier = auth::random_token();
    let request = AuthorizationInput {
        client_id: "synthetic-account".into(),
        redirect_uri: "https://synthetic.example.invalid/callback".into(),
        response_type: "code".into(),
        scope: format!("openid {scope}"),
        resource: "https://synthetic.example.invalid/api".into(),
        state: auth::random_token(),
        nonce: auth::random_token(),
        code_challenge: URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())),
        code_challenge_method: "S256".into(),
        dpop_jkt: None,
        max_age: None,
        prompt: None,
    }
    .validate(&clients)?;
    let handle = store::create_request(db, &request, RequestKind::Authorization).await?;
    let code = codes::authorize(db, &clients, &handle, session_token).await?;
    let grant = codes::exchange(
        db,
        &clients,
        CodeExchange {
            code: &code.code,
            client_id: request.client_id(),
            redirect_uri: request.redirect_uri(),
            verifier: &verifier,
            verified_dpop_jkt: None,
        },
    )
    .await?;
    Ok(service
        .issue_grant(db, &clients, &grant)
        .await?
        .access_token)
}
#[derive(Debug, Serialize)]
pub struct SyntheticTokenResult {
    pub principal_id: Uuid,
    pub session_id: Uuid,
    pub algorithm: &'static str,
    pub key_id: String,
    pub expires_in_seconds: u64,
    pub scope: String,
    pub amr: Vec<String>,
    pub active_before_revocation: bool,
    pub inactive_for_wrong_audience: bool,
    pub inactive_after_revocation: bool,
}

pub async fn run_synthetic_smoke(
    db: &PgPool,
    service: &TokenService,
    email: &str,
    password: &str,
    audience: &str,
) -> anyhow::Result<SyntheticTokenResult> {
    let principal_id = auth::create_synthetic_identity(db, email, password).await?;
    let session_token = auth::authenticate(db, email, password).await?;
    let session_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM identity_sessions WHERE token_hash = $1 AND principal_id = $2",
    )
    .bind(auth::hash_token(&session_token))
    .bind(principal_id)
    .fetch_one(db)
    .await?;
    let clients = clients(audience)?;
    let access_token = issue_for_session(
        service,
        db,
        &session_token,
        audience,
        "account:read account:write account:export account:close",
    )
    .await?;
    let active_before_revocation = service
        .introspect(db, &clients, &access_token, audience)
        .await?
        .is_some();
    let inactive_for_wrong_audience = service
        .introspect(db, &clients, &access_token, "unregistered-audience")
        .await?
        .is_none();
    sqlx::query("UPDATE identity_sessions SET revoked_at = clock_timestamp() WHERE id = $1")
        .bind(session_id)
        .execute(db)
        .await?;
    let inactive_after_revocation = service
        .introspect(db, &clients, &access_token, audience)
        .await?
        .is_none();
    let jwks = service.jwks();
    let key = jwks
        .keys
        .iter()
        .find(|key| {
            Some(key.kid.as_str())
                == jsonwebtoken::decode_header(&access_token)
                    .ok()
                    .and_then(|h| h.kid)
                    .as_deref()
        })
        .ok_or_else(|| anyhow::anyhow!("issued token key not published"))?;
    let claims = service.verify(&access_token, audience)?;
    record_synthetic_proof(
        db,
        principal_id,
        session_id,
        &claims,
        active_before_revocation,
        inactive_after_revocation,
    )
    .await?;
    Ok(SyntheticTokenResult {
        principal_id,
        session_id,
        algorithm: key.alg,
        key_id: key.kid.clone(),
        expires_in_seconds: ACCESS_TOKEN_TTL_SECONDS,
        scope: claims.scope,
        amr: claims.amr,
        active_before_revocation,
        inactive_for_wrong_audience,
        inactive_after_revocation,
    })
}

async fn record_synthetic_proof(
    db: &PgPool,
    principal_id: Uuid,
    session_id: Uuid,
    claims: &AccessTokenClaims,
    active_before_revocation: bool,
    inactive_after_revocation: bool,
) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO identity_audit_events(id,principal_id,actor_principal_id,event_type,correlation_id,details) VALUES($1,$2,$2,'identity.token.synthetic_proven',$3,jsonb_build_object('session_id',$4,'audience',$5,'scope',$6,'amr',$7,'active_before_revocation',$8,'inactive_after_revocation',$9))")
        .bind(Uuid::new_v4())
        .bind(principal_id)
        .bind(Uuid::new_v4())
        .bind(session_id)
        .bind(&claims.aud)
        .bind(&claims.scope)
        .bind(serde_json::to_value(&claims.amr)?)
        .bind(active_before_revocation)
        .bind(inactive_after_revocation)
        .execute(db)
        .await?;
    Ok(())
}
