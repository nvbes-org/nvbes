use super::{OAuthError, ProtocolError};
use axum::{
    Json,
    http::{HeaderMap, header},
    response::{IntoResponse, Redirect, Response},
};

const PRIVATE_NAVIGATION: [(&str, &str); 4] = [
    ("cache-control", "no-store"),
    ("pragma", "no-cache"),
    ("referrer-policy", "no-referrer"),
    ("vary", "accept"),
];

fn callback(
    redirect_uri: &str,
    key: &str,
    value: &str,
    state: &str,
) -> Result<reqwest::Url, ProtocolError> {
    // The caller obtains this URI from the validated, server-held interaction.
    let mut url = reqwest::Url::parse(redirect_uri)
        .map_err(|_| ProtocolError::OAuth(OAuthError::InvalidRequest))?;
    url.query_pairs_mut()
        .append_pair(key, value)
        .append_pair("state", state);
    Ok(url)
}

pub(super) fn redirect_with_result(
    redirect_uri: &str,
    key: &str,
    value: &str,
    state: &str,
) -> Result<Response, ProtocolError> {
    let url = callback(redirect_uri, key, value, state)?;
    // A 303 never forwards the hosted POST body to the OAuth client.
    Ok((PRIVATE_NAVIGATION, Redirect::to(url.as_str())).into_response())
}

/// Hosted fetch clients explicitly opt into JSON, then perform a top-level navigation.
/// This response is protected by the same interaction, Origin and CSRF checks as a redirect.
pub(super) fn interaction_result(
    headers: &HeaderMap,
    redirect_uri: &str,
    key: &str,
    value: &str,
    state: &str,
) -> Result<Response, ProtocolError> {
    let mut accept = headers.get_all(header::ACCEPT).iter();
    let json = accept
        .next()
        .is_some_and(|value| value.as_bytes().eq_ignore_ascii_case(b"application/json"))
        && accept.next().is_none();
    if !json {
        return redirect_with_result(redirect_uri, key, value, state);
    }
    let url = callback(redirect_uri, key, value, state)?;
    Ok((
        PRIVATE_NAVIGATION,
        Json(serde_json::json!({"redirect_uri": url.as_str()})),
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::to_bytes, http::StatusCode};

    #[tokio::test]
    async fn json_and_redirect_preserve_callback_encoding_and_disable_caching() {
        for (key, value) in [("code", "code+&="), ("error", "access_denied")] {
            for json in [false, true] {
                let mut headers = HeaderMap::new();
                if json {
                    headers.insert(header::ACCEPT, "application/json".parse().unwrap());
                }
                let response = interaction_result(
                    &headers,
                    "https://account.example/callback?existing=yes",
                    key,
                    value,
                    "state+&=",
                )
                .unwrap();
                assert_eq!(response.headers()["cache-control"], "no-store");
                assert_eq!(response.headers()["referrer-policy"], "no-referrer");
                assert_eq!(response.headers()["vary"], "accept");
                let destination = if json {
                    assert_eq!(response.status(), StatusCode::OK);
                    assert!(!response.headers().contains_key(header::LOCATION));
                    let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
                    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                    body["redirect_uri"].as_str().unwrap().to_owned()
                } else {
                    assert_eq!(response.status(), StatusCode::SEE_OTHER);
                    response.headers()[header::LOCATION]
                        .to_str()
                        .unwrap()
                        .to_owned()
                };
                let url = reqwest::Url::parse(&destination).unwrap();
                let params: std::collections::HashMap<_, _> = url.query_pairs().collect();
                assert_eq!(params[key], value);
                assert_eq!(params["state"], "state+&=");
                assert_eq!(params["existing"], "yes");
            }
        }
    }
}
