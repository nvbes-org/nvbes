use super::*;

#[tokio::test]
async fn json_consent_requires_the_bound_proof_and_cannot_be_replayed() {
    for action in ["approve", "deny"] {
        let f = Fixture::new().await;
        let (_, interaction) = f.authorize().await;
        let body = serde_json::json!({"interaction": interaction["interaction"]}).to_string();
        let csrf = interaction["csrf_token"].as_str().unwrap();
        let request = |origin: &str, csrf: &str| {
            Request::post(format!("/oauth/authorize/{action}"))
                .header("origin", origin)
                .header("cookie", f.cookie())
                .header("x-csrf-token", csrf)
                .header("content-type", "application/json")
                .header("accept", "application/json")
                .body(Body::from(body.clone()))
                .unwrap()
        };
        for (origin, proof) in [
            ("https://attacker.example", csrf),
            ("https://identity.example", "invalid"),
        ] {
            let response = f.app.clone().oneshot(request(origin, proof)).await.unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
        let response = f
            .app
            .clone()
            .oneshot(request("https://identity.example", csrf))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!response.headers().contains_key("location"));
        assert_eq!(response.headers()["cache-control"], "no-store");
        let bytes = to_bytes(response.into_body(), 8192).await.unwrap();
        let result: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let destination = reqwest::Url::parse(result["redirect_uri"].as_str().unwrap()).unwrap();
        let registered = reqwest::Url::parse(&test_fixtures::input().redirect_uri).unwrap();
        assert_eq!(destination.origin(), registered.origin());
        assert_eq!(destination.path(), registered.path());
        let parameters: std::collections::HashMap<_, _> = destination.query_pairs().collect();
        assert_eq!(parameters["state"], test_fixtures::input().state);
        if action == "approve" {
            assert!(parameters.contains_key("code"));
        } else {
            assert_eq!(parameters["error"], "access_denied");
        }
        let replay = f
            .app
            .clone()
            .oneshot(request("https://identity.example", csrf))
            .await
            .unwrap();
        assert_eq!(replay.status(), StatusCode::BAD_REQUEST);
    }
}
