use std::time::Duration;

pub(crate) async fn issue_machine_token(
    base_url: &str,
    client_id: &str,
    client_secret: &str,
) -> String {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("http client should build");
    let request = http
        .post(format!("{base_url}/oauth/token"))
        .basic_auth(client_id, Some(client_secret))
        .json(&serde_json::json!({
            "grant_type": "client_credentials",
            "scope": "drive.files.read drive.workspace.read",
            "audience": "nvbes-cloud-service",
        }));
    let token_response = nvbes_core::trace_context::with_fresh_trace_headers(request)
        .send()
        .await
        .expect("token endpoint should respond");

    assert!(token_response.status().is_success());
    let token_body = token_response
        .json::<serde_json::Value>()
        .await
        .expect("token body should be valid json");
    token_body["access_token"]
        .as_str()
        .expect("access token should exist")
        .to_string()
}
