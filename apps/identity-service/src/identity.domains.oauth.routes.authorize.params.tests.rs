use super::build_params_from_map;
use serde_json::{Map, Value, json};

#[test]
fn accepts_code_response_type_from_pushed_request() {
    let params = pushed_request_with_response_type(json!("code"));

    let resolved = build_params_from_map(&params).expect("supported response type");

    assert_eq!(
        resolved.redirect_uri,
        "https://account.example/oauth/callback"
    );
}

#[test]
fn rejects_unsupported_response_type_from_pushed_request() {
    let params = pushed_request_with_response_type(json!("token"));

    let error = build_params_from_map(&params).expect_err("unsupported response type");

    assert_eq!(error.code, "invalid_response_type");
}

fn pushed_request_with_response_type(response_type: Value) -> Map<String, Value> {
    Map::from_iter([
        ("response_type".to_string(), response_type),
        (
            "redirect_uri".to_string(),
            json!("https://account.example/oauth/callback"),
        ),
    ])
}
