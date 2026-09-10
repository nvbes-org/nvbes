use super::*;

const START: &str = "/oauth/session/totp/enrollment/start";
const CONFIRM: &str = "/oauth/session/totp/enrollment/confirm";
const ORIGIN: &str = "https://identity.example";

#[tokio::test]
async fn totp_confirmation_rejects_positional_duplicate_and_unknown_fields() {
    let f = Fixture::new().await;
    let (_, auth) = f.authorize().await;
    let csrf = auth["session_csrf_token"].as_str().unwrap();
    let (_, _, pending) = f.post(START, csrf, "{}", ORIGIN).await;
    let id = pending["factor_id"].as_str().unwrap();
    let code = generate_totp_code(
        pending["secret_base32"].as_str().unwrap(),
        current_counter(chrono::Utc::now()),
    );
    for body in [
        serde_json::json!([id, code]).to_string(),
        format!(r#"{{"factor_id":"{id}","code":"{code}","code":"{code}"}}"#),
        serde_json::json!({"factor_id":id,"code":code,"extra":true}).to_string(),
    ] {
        assert_eq!(
            f.post(CONFIRM, csrf, &body, ORIGIN).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        f.post(
            CONFIRM,
            csrf,
            &serde_json::json!({"factor_id":id,"code":code}).to_string(),
            ORIGIN
        )
        .await
        .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn totp_http_enrollment_confirms_and_rejects_replay() {
    let f = Fixture::new().await;
    let (_, auth) = f.authorize().await;
    let csrf = auth["session_csrf_token"].as_str().unwrap();
    let (status, _, pending) = f.post(START, csrf, "{}", ORIGIN).await;
    assert_eq!(status, StatusCode::OK);
    let secret = pending["secret_base32"].as_str().unwrap();
    let code = generate_totp_code(secret, current_counter(chrono::Utc::now()));
    let body = serde_json::json!({"factor_id":pending["factor_id"],"code":code}).to_string();
    let (status, _, confirmed) = f.post(CONFIRM, csrf, &body, ORIGIN).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(confirmed["enrolled"], true);
    assert_eq!(confirmed["step_up"], true);
    assert!(confirmed.get("secret_base32").is_none());
    assert_eq!(
        f.post(CONFIRM, csrf, &body, ORIGIN).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        f.post(
            "/oauth/session/step-up/totp",
            csrf,
            &serde_json::json!({"code":code}).to_string(),
            ORIGIN
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn totp_http_rejects_origin_csrf_and_counts_invalid_attempts() {
    let f = Fixture::new().await;
    let (_, auth) = f.authorize().await;
    let csrf = auth["session_csrf_token"].as_str().unwrap();
    for path in [
        START,
        CONFIRM,
        "/oauth/session/totp/factors/list",
        "/oauth/session/totp/factors/revoke",
    ] {
        assert_eq!(
            f.post(path, csrf, "{", "https://other.example").await.0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            f.post(path, &random_secret(), "{", ORIGIN).await.0,
            StatusCode::FORBIDDEN
        );
    }
    for body in [
        "null".into(),
        "[]".into(),
        "{".into(),
        "{\"unknown\":true}".into(),
        "x".repeat(4097),
    ] {
        assert_eq!(
            f.post(START, csrf, &body, ORIGIN).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    let (status, headers, _) = f.post(START, csrf, "{}", ORIGIN).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(headers["retry-after"], "600");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_auth_factors")
        .fetch_one(&f.db)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn totp_management_http_lists_owned_metadata_and_preserves_last_factor() {
    let f = Fixture::new().await;
    let (_, auth) = f.authorize().await;
    let csrf = auth["session_csrf_token"].as_str().unwrap();
    let list = "/oauth/session/totp/factors/list";
    let revoke = "/oauth/session/totp/factors/revoke";
    assert_eq!(
        f.post(list, csrf, "{}", ORIGIN).await.2,
        serde_json::json!([])
    );
    for body in ["[]", "null", "{\"unknown\":true}"] {
        assert_eq!(
            f.post(list, csrf, body, ORIGIN).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    let (_, _, pending) = f.post(START, csrf, "{}", ORIGIN).await;
    let code = generate_totp_code(
        pending["secret_base32"].as_str().unwrap(),
        current_counter(chrono::Utc::now()),
    );
    let body = serde_json::json!({"factor_id":pending["factor_id"],"code":code}).to_string();
    assert_eq!(f.post(CONFIRM, csrf, &body, ORIGIN).await.0, StatusCode::OK);
    let (status, headers, rows) = f.post(list, csrf, "{}", ORIGIN).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["cache-control"], "no-store");
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0].as_object().unwrap().len(), 2);
    assert_eq!(rows[0]["id"], pending["factor_id"]);
    let (status, _, error) = f
        .post(
            revoke,
            csrf,
            &serde_json::json!({"factor_id":pending["factor_id"]}).to_string(),
            ORIGIN,
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"], "last_strong_factor");
}
