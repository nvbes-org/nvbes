use axum::http::{HeaderMap, Uri};

use crate::http::error::AppError;

pub fn resolve_authuser(uri: &Uri, headers: &HeaderMap) -> Result<String, AppError> {
    crate::http::authuser::from_uri_and_headers(uri, headers)
}
