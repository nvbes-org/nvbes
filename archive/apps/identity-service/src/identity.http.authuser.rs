use crate::http::error::AppError;
use axum::http::{HeaderMap, Uri};

pub fn from_uri_and_headers(uri: &Uri, headers: &HeaderMap) -> Result<String, AppError> {
    let raw = uri
        .query()
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
        });

    Ok(normalize(raw.as_deref())?.to_string())
}

pub fn normalize(value: Option<&str>) -> Result<&str, AppError> {
    match value {
        None | Some("") => Ok("0"),
        Some(value) if is_valid(value) => Ok(value),
        Some(_) => Err(AppError::bad_request(
            "invalid_authuser",
            "Account selector must be a numeric authuser index.",
        )),
    }
}

fn is_valid(value: &str) -> bool {
    value.len() <= 3 && value.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::{from_uri_and_headers, normalize};
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn normalize_accepts_small_numeric_indexes() {
        assert_eq!(normalize(None).expect("default authuser"), "0");
        assert_eq!(normalize(Some("12")).expect("numeric authuser"), "12");
    }

    #[test]
    fn normalize_rejects_cookie_name_injection() {
        let error =
            normalize(Some("1; session=evil")).expect_err("non numeric authuser must be rejected");

        assert_eq!(error.code, "invalid_authuser");
    }

    #[test]
    fn from_uri_prefers_query_over_header() {
        let uri = "/auth/me?authuser=2".parse().expect("valid uri");
        let mut headers = HeaderMap::new();
        headers.insert("X-Auth-User", HeaderValue::from_static("1"));

        assert_eq!(from_uri_and_headers(&uri, &headers).expect("authuser"), "2");
    }
}
