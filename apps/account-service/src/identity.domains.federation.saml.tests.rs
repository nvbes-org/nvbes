use axum::{Router, routing::get};
use chrono::Utc;
use tokio::net::TcpListener;
use uuid::Uuid;

use super::{
    fetch_saml_metadata, find_assertion_by_id, first_assertion,
    saml_subject::validate_subject_confirmation,
};
use crate::domains::federation::types::FederatedIdentityProviderRecord;
use serde_json::json;

async fn spawn_server(build_router: impl FnOnce(String) -> Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test server should bind");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{}", addr);
    let router = build_router(base_url.clone());
    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("test server should run");
    });
    base_url
}

fn provider(base_url: &str) -> FederatedIdentityProviderRecord {
    FederatedIdentityProviderRecord {
        id: Uuid::new_v4(),
        provider_type: "saml".to_string(),
        provider_family: "custom".to_string(),
        name: "Acme SAML".to_string(),
        client_id: Some("sp-entity".to_string()),
        issuer: Some("https://idp.example.com".to_string()),
        metadata_url: Some(format!("{}/metadata", base_url)),
        status: "active".to_string(),
        sp_entity_id: None,
        attribute_mapping: json!({}),
        encryption_cert_pem: None,
        require_signed_assertions: false,
        require_signed_responses: false,
        created_at: Utc::now(),
    }
}

#[tokio::test]
async fn fetch_saml_metadata_parses_entity_id_and_sso_url() {
    let base_url = spawn_server(|base_url| {
        let acs_url = format!("{}/acs", base_url);
        Router::new().route(
            "/metadata",
            get(move || {
                let acs_url = acs_url.clone();
                async move {
                    format!(
                        r#"<EntityDescriptor entityID="https://idp.example.com">
  <IDPSSODescriptor>
    <SingleSignOnService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST" Location="{acs}"/>
  </IDPSSODescriptor>
</EntityDescriptor>"#,
                        acs = acs_url
                    )
                }
            }),
        )
    })
    .await;

    let metadata = fetch_saml_metadata(&provider(&base_url), false)
        .await
        .expect("metadata should parse");

    assert_eq!(metadata.entity_id, "https://idp.example.com");
    let expected_sso_url = format!("{}/acs", base_url);
    assert_eq!(metadata.sso_url.as_deref(), Some(expected_sso_url.as_str()));
    assert!(metadata.signing_certificates.is_empty());
}

#[test]
fn validate_subject_confirmation_accepts_bearer_subject() {
    let acs_url = "https://sp.example.com/saml/acs";
    let xml = format!(
        r#"<Assertion ID="assertion-123">
  <Subject>
    <NameID>alice@example.com</NameID>
    <SubjectConfirmation Method="urn:oasis:names:tc:SAML:2.0:cm:bearer">
      <SubjectConfirmationData Recipient="{acs_url}" />
    </SubjectConfirmation>
  </Subject>
</Assertion>"#
    );
    let doc = roxmltree::Document::parse(&xml).expect("assertion should parse");
    let assertion = doc
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "Assertion")
        .expect("assertion should exist");

    let subject = validate_subject_confirmation(&assertion, acs_url)
        .expect("subject confirmation should validate");

    assert_eq!(subject.recipient.as_deref(), Some(acs_url));
    assert!(subject.in_response_to.is_none());
}

#[test]
fn find_assertion_by_id_uses_signed_assertion_not_first_assertion() {
    let xml = r#"<Response>
  <Assertion ID="attacker">
    <Subject><NameID>attacker@example.com</NameID></Subject>
  </Assertion>
  <Assertion ID="signed-assertion">
    <Subject><NameID>alice@example.com</NameID></Subject>
  </Assertion>
</Response>"#;
    let doc = roxmltree::Document::parse(xml).expect("response should parse");

    let assertion = find_assertion_by_id(&doc, "signed-assertion")
        .expect("signed assertion should be selected by ID");
    let name_id = assertion
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "NameID")
        .and_then(|node| node.text());

    assert_eq!(name_id, Some("alice@example.com"));
}

#[test]
fn first_assertion_rejects_multiple_unsigned_assertions() {
    let xml = r#"<Response>
  <Assertion ID="first" />
  <Assertion ID="second" />
</Response>"#;
    let doc = roxmltree::Document::parse(xml).expect("response should parse");

    let error = first_assertion(&doc).expect_err("multiple unsigned assertions should fail");

    assert_eq!(
        error.message,
        "Unsigned SAML responses with multiple assertions are not accepted."
    );
}

#[test]
fn validate_subject_confirmation_rejects_missing_recipient() {
    let xml = r#"<Assertion ID="assertion-123">
  <Subject>
    <NameID>alice@example.com</NameID>
    <SubjectConfirmation Method="urn:oasis:names:tc:SAML:2.0:cm:bearer">
      <SubjectConfirmationData />
    </SubjectConfirmation>
  </Subject>
</Assertion>"#;
    let doc = roxmltree::Document::parse(xml).expect("assertion should parse");
    let assertion = doc
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "Assertion")
        .expect("assertion should exist");

    let error = validate_subject_confirmation(&assertion, "https://sp.example.com/saml/acs")
        .expect_err("missing recipient should fail");

    assert_eq!(error.code, "saml_recipient_missing");
}

#[test]
fn validate_subject_confirmation_rejects_wrong_recipient() {
    let expected_acs_url = "https://sp.example.com/saml/acs";
    let xml = r#"<Assertion ID="assertion-123">
  <Subject>
    <NameID>alice@example.com</NameID>
    <SubjectConfirmation Method="urn:oasis:names:tc:SAML:2.0:cm:bearer">
      <SubjectConfirmationData Recipient="https://idp.example.com/sso" />
    </SubjectConfirmation>
  </Subject>
</Assertion>"#;
    let doc = roxmltree::Document::parse(xml).expect("assertion should parse");
    let assertion = doc
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "Assertion")
        .expect("assertion should exist");

    let error = validate_subject_confirmation(&assertion, expected_acs_url)
        .expect_err("wrong recipient should fail");

    assert_eq!(error.code, "saml_recipient_mismatch");
}
