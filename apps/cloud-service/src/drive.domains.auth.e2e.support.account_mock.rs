use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use openssl::rsa::Rsa;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Row, postgres::PgPool};
use tokio::net::TcpListener;
use uuid::Uuid;

const TEST_CLOUD_AUDIENCE: &str = "nvbes-cloud-service";

#[derive(Clone)]
pub(crate) struct MockAccountState {
    pool: PgPool,
    key: MockSigningKey,
    issuer: String,
}

#[derive(Clone)]
struct MockSigningKey {
    kid: String,
    private_key_pem: Vec<u8>,
    n: String,
    e: String,
}

#[derive(Debug, Deserialize)]
struct TokenRequest {
    scope: Option<String>,
    audience: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TestTokenClaims {
    sub: String,
    workspace_id: Option<String>,
    tenant_id: Option<String>,
    organization_id: Option<String>,
    token_type: String,
    scope: String,
    role: String,
    amr: Vec<String>,
    client_id: Option<String>,
    iss: String,
    aud: String,
    exp: i64,
    iat: i64,
    nbf: i64,
    jti: String,
    sid: String,
}

pub(crate) async fn identity_state(pool: &PgPool) -> MockAccountState {
    MockAccountState {
        pool: pool.clone(),
        key: MockSigningKey::generate(),
        issuer: String::new(),
    }
}

pub(crate) async fn spawn_identity_server(mut state: MockAccountState) -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test server should bind");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{addr}");
    state.issuer = base_url.clone();
    let router = Router::new()
        .route("/.well-known/jwks.json", get(jwks))
        .route("/oauth/token", post(issue_token))
        .route("/oauth/introspect", post(introspect_token))
        .with_state(state);
    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("test server should run");
    });
    base_url
}

async fn jwks(State(state): State<MockAccountState>) -> Json<serde_json::Value> {
    Json(json!({
        "keys": [{
            "kty": "RSA",
            "kid": state.key.kid,
            "alg": "PS256",
            "use": "sig",
            "n": state.key.n,
            "e": state.key.e,
        }]
    }))
}

async fn issue_token(
    State(state): State<MockAccountState>,
    headers: HeaderMap,
    Json(body): Json<TokenRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let (client_id, client_secret) = basic_auth(&headers)?;
    let row = account_client_row(&state.pool, &client_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let secret_hash: String = row.get("client_secret_hash");
    nvbes_product_account::oauth::verify_client_secret(&client_secret, &secret_hash)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let now = Utc::now().timestamp();
    let jti = Uuid::new_v4().to_string();
    let claims = TestTokenClaims {
        sub: row.get::<Uuid, _>("principal_id").to_string(),
        workspace_id: Some(row.get::<Uuid, _>("workspace_id").to_string()),
        tenant_id: Some(row.get::<Uuid, _>("tenant_id").to_string()),
        organization_id: None,
        token_type: "access".to_string(),
        scope: body
            .scope
            .unwrap_or_else(|| "drive.files.read drive.workspace.read".to_string()),
        role: "owner".to_string(),
        amr: vec!["m2m".to_string()],
        client_id: Some(client_id),
        iss: state.issuer.clone(),
        aud: body
            .audience
            .unwrap_or_else(|| TEST_CLOUD_AUDIENCE.to_string()),
        exp: now + 3600,
        iat: now,
        nbf: now,
        sid: jti.clone(),
        jti,
    };

    Ok(Json(json!({
        "access_token": sign_claims(&state.key, &claims)?,
        "token_type": "Bearer",
        "expires_in": 3600,
        "scope": claims.scope,
    })))
}

async fn introspect_token(
    State(state): State<MockAccountState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let (client_id, client_secret) = basic_auth(&headers)?;
    let row = account_client_row(&state.pool, &client_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let secret_hash: String = row.get("client_secret_hash");
    nvbes_product_account::oauth::verify_client_secret(&client_secret, &secret_hash)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let token = body
        .get("token")
        .and_then(serde_json::Value::as_str)
        .ok_or(StatusCode::BAD_REQUEST)?;
    let claims = decode_test_claims(&state.key, token)?;
    let token_row =
        account_client_row(&state.pool, claims.client_id.as_deref().unwrap_or_default())
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let client_revoked = token_row
        .try_get::<Option<chrono::DateTime<Utc>>, _>("revoked_at")
        .ok()
        .flatten()
        .is_some();
    let principal_status: String = token_row.get("principal_status");

    Ok(Json(json!({
        "active": !client_revoked && principal_status == "active",
        "scope": claims.scope,
        "client_id": claims.client_id,
        "principal_type": "service_account",
        "token_type": "access_token",
        "sub": claims.sub,
        "role": claims.role,
        "tenant_id": claims.tenant_id,
        "organization_id": claims.organization_id,
        "workspace_id": claims.workspace_id,
        "email_verified": true,
        "name": "Drive E2E Robot",
        "acr": "aal1",
        "amr": claims.amr,
        "auth_time": claims.iat,
        "jti": claims.jti,
        "sid": claims.sid,
        "exp": claims.exp,
        "iat": claims.iat,
        "nbf": claims.nbf,
        "network_valid": true,
    })))
}

async fn account_client_row(
    pool: &PgPool,
    client_id: &str,
) -> Result<sqlx::postgres::PgRow, sqlx::Error> {
    sqlx::query(
        r#"
        SELECT
          oauth_clients.client_secret_hash,
          oauth_clients.revoked_at,
          oauth_clients.tenant_id,
          oauth_clients.owner_scope_id AS workspace_id,
          service_accounts.principal_id,
          principals.status::text AS principal_status
        FROM oauth_clients
        JOIN service_accounts ON service_accounts.client_id = oauth_clients.client_id
        JOIN principals ON principals.id = service_accounts.principal_id
        WHERE oauth_clients.client_id = $1
        "#,
    )
    .bind(client_id)
    .fetch_one(pool)
    .await
}

fn sign_claims(key: &MockSigningKey, claims: &TestTokenClaims) -> Result<String, StatusCode> {
    let mut header = Header::new(Algorithm::PS256);
    header.kid = Some(key.kid.clone());
    encode(
        &header,
        claims,
        &EncodingKey::from_rsa_pem(&key.private_key_pem)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn decode_test_claims(key: &MockSigningKey, token: &str) -> Result<TestTokenClaims, StatusCode> {
    let mut validation = Validation::new(Algorithm::PS256);
    validation.set_issuer(&[&state.issuer]);
    validation.set_audience(&[TEST_CLOUD_AUDIENCE]);
    decode::<TestTokenClaims>(
        token,
        &DecodingKey::from_rsa_components(&key.n, &key.e)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|_| StatusCode::UNAUTHORIZED)
}

fn basic_auth(headers: &HeaderMap) -> Result<(String, String), StatusCode> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let encoded = value
        .strip_prefix("Basic ")
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let decoded = STANDARD
        .decode(encoded)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    let decoded = String::from_utf8(decoded).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let (client_id, client_secret) = decoded.split_once(':').ok_or(StatusCode::UNAUTHORIZED)?;
    Ok((client_id.to_string(), client_secret.to_string()))
}

impl MockSigningKey {
    fn generate() -> Self {
        let rsa = Rsa::generate(2048).expect("test account RSA key should generate");
        Self {
            kid: format!("drive-e2e-{}", Uuid::new_v4()),
            private_key_pem: rsa
                .private_key_to_pem()
                .expect("test account RSA key should encode"),
            n: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(rsa.n().to_vec()),
            e: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(rsa.e().to_vec()),
        }
    }
}
