use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::Utc;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::http::error::AppError;

#[derive(Debug)]
pub struct AuthnRequestInput {
    pub sp_entity_id: String,
    pub acs_url: String,
    pub idp_sso_url: String,
    pub name_id_format: Option<String>,
    pub relay_state: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthnRequestOutput {
    pub authn_request: String,
    pub redirect_url: String,
    pub request_id: String,
    pub relay_state: Option<String>,
}

pub fn build_authn_request_xml(
    request_id: &str,
    sp_entity_id: &str,
    acs_url: &str,
    name_id_format: &str,
) -> String {
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    format!(
        r#"<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol"
  xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion"
  ID="{request_id}"
  Version="2.0"
  IssueInstant="{now}"
  ProtocolBinding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"
  AssertionConsumerServiceURL="{acs_url}"
  Destination="{sp_entity_id}">
  <saml:Issuer>{sp_entity_id}</saml:Issuer>
  <samlp:NameIDPolicy Format="{name_id_format}" AllowCreate="true"/>
</samlp:AuthnRequest>"#,
    )
}

pub fn encode_authn_request(xml: &str) -> String {
    let compressed = deflate_raw(xml.as_bytes());
    STANDARD.encode(&compressed)
}

fn deflate_raw(input: &[u8]) -> Vec<u8> {
    use flate2::Compression;
    use flate2::write::DeflateEncoder;
    use std::io::Write;

    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(input).unwrap_or_default();
    encoder.finish().unwrap_or_default()
}

pub fn build_redirect_url(
    idp_sso_url: &str,
    encoded_request: &str,
    relay_state: Option<&str>,
) -> String {
    let mut url = format!(
        "{}?SAMLRequest={}",
        idp_sso_url,
        urlencoding::encode(encoded_request)
    );

    if let Some(rs) = relay_state {
        url.push_str("&RelayState=");
        url.push_str(&urlencoding::encode(rs));
    }

    url
}

pub async fn store_pending_request(
    db: &PgPool,
    tenant_id: Uuid,
    provider_id: Uuid,
    request_id: &str,
    relay_state: Option<&str>,
) -> Result<(), AppError> {
    let expires_at = Utc::now() + chrono::Duration::minutes(10);
    let id = Uuid::parse_str(request_id).unwrap_or_else(|_| Uuid::new_v4());

    sqlx::query(
        "INSERT INTO saml_pending_requests (id, tenant_id, provider_id, relay_state, expires_at) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(provider_id)
    .bind(relay_state)
    .bind(expires_at)
    .execute(db)
    .await?;

    Ok(())
}

pub async fn generate_authn_request(
    db: &PgPool,
    tenant_id: Uuid,
    provider_id: Uuid,
    input: &AuthnRequestInput,
) -> Result<AuthnRequestOutput, AppError> {
    let request_id = Uuid::new_v4().to_string();
    let name_id_format = input
        .name_id_format
        .as_deref()
        .unwrap_or("urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress");

    let xml = build_authn_request_xml(
        &request_id,
        &input.sp_entity_id,
        &input.acs_url,
        name_id_format,
    );
    let encoded = encode_authn_request(&xml);
    let redirect_url =
        build_redirect_url(&input.idp_sso_url, &encoded, input.relay_state.as_deref());

    store_pending_request(
        db,
        tenant_id,
        provider_id,
        &request_id,
        input.relay_state.as_deref(),
    )
    .await?;

    Ok(AuthnRequestOutput {
        authn_request: encoded,
        redirect_url,
        request_id,
        relay_state: input.relay_state.clone(),
    })
}
