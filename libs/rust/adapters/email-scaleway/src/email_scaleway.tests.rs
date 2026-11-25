use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode},
    routing::post,
};
use chrono::{Duration, Utc};
use nvbes_email::{EmailAddress, EmailFailureClass, EmailMessage, EmailSender};
use serde_json::{Value, json};

use super::ScalewayEmailClient;

async fn accepted(headers: HeaderMap, Json(body): Json<Value>) -> Json<Value> {
    assert_eq!(headers.get("x-auth-token").unwrap(), "test-secret");
    assert_eq!(body["project_id"], "test-project");
    assert_eq!(body["to"][0]["email"], "recipient@nvbes.fr");
    assert_eq!(body["additional_headers"][0]["key"], "Message-ID");
    assert!(body["send_before"].as_str().is_some());
    Json(json!({ "emails": [{ "id": "provider-email-1" }] }))
}

async fn rejected() -> StatusCode {
    StatusCode::TOO_MANY_REQUESTS
}

async fn server(router: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (format!("http://{address}/emails"), task)
}

fn message() -> EmailMessage {
    EmailMessage {
        from: EmailAddress {
            email: "no-reply@notify.nvbes.eu".to_string(),
            name: Some("nvbes".to_string()),
        },
        to: vec![EmailAddress {
            email: "recipient@nvbes.fr".to_string(),
            name: None,
        }],
        subject: "Security notification".to_string(),
        text_body: Some("Security notification".to_string()),
        html_body: Some("<p>Security notification</p>".to_string()),
        headers: vec![(
            "Message-ID".to_string(),
            "<stable@notify.nvbes.eu>".to_string(),
        )],
    }
}

#[tokio::test]
async fn adapter_sends_multipart_payload_deadline_and_stable_headers() {
    let (endpoint, task) = server(Router::new().route("/emails", post(accepted))).await;
    let client = ScalewayEmailClient::for_test(endpoint);
    let result = client
        .send_message_before(&message(), Utc::now() + Duration::minutes(5))
        .await
        .unwrap();
    assert_eq!(result.provider_email_id, "provider-email-1");
    task.abort();
}

#[tokio::test]
async fn adapter_classifies_rate_limits_as_transient() {
    let (endpoint, task) = server(Router::new().route("/emails", post(rejected))).await;
    let error = ScalewayEmailClient::for_test(endpoint)
        .send_message(&message())
        .await
        .unwrap_err();
    assert_eq!(error.failure_class(), EmailFailureClass::Transient);
    task.abort();
}

#[test]
fn constructor_sets_up_endpoint() {
    let client = ScalewayEmailClient::new(
        "test-secret".to_string(),
        "test-project".to_string(),
        "fr-par",
    );
    assert_eq!(
        client.api_url,
        "https://api.scaleway.com/transactional-email/v1alpha1/regions/fr-par/emails"
    );
    assert_eq!(client.secret_key, "test-secret");
    assert_eq!(client.project_id, "test-project");
}

async fn accepted_simple(_headers: HeaderMap, Json(body): Json<Value>) -> Json<Value> {
    assert_eq!(body["to"][0]["name"], "Alice");
    Json(json!({ "emails": [{ "id": "provider-email-simple" }] }))
}

async fn empty_emails_response() -> Json<Value> {
    Json(json!({ "emails": [] }))
}

#[tokio::test]
async fn adapter_sends_message_without_deadline_and_with_recipient_name() {
    let (endpoint, task) = server(Router::new().route("/emails", post(accepted_simple))).await;
    let client = ScalewayEmailClient::for_test(endpoint);
    let mut msg = message();
    msg.to[0].name = Some("Alice".to_string());
    let result = client.send_message(&msg).await.unwrap();
    assert_eq!(result.provider_email_id, "provider-email-simple");
    task.abort();
}

#[tokio::test]
async fn adapter_handles_empty_email_list_response() {
    let (endpoint, task) =
        server(Router::new().route("/emails", post(empty_emails_response))).await;
    let client = ScalewayEmailClient::for_test(endpoint);
    let error = client.send_message(&message()).await.unwrap_err();
    match error {
        nvbes_email::EmailError::Api { status, message } => {
            assert_eq!(status, 200);
            assert!(message.contains("no email"));
        }
        other => panic!("expected EmailError::Api, got {other:?}"),
    }
    task.abort();
}
