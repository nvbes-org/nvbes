use axum::http::{HeaderMap, Uri};

pub fn resolve_authuser(uri: &Uri, headers: &HeaderMap) -> String {
    uri.query()
        .and_then(|query| {
            url::form_urlencoded::parse(query.as_bytes())
                .find(|(key, _)| key == "authuser")
                .map(|(_, value)| value.into_owned())
        })
        .or_else(|| {
            headers
                .get("X-Auth-User")
                .and_then(|header| header.to_str().ok())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "0".to_string())
}
