use super::{BrowserError, BrowserSecurity, read_cookie};
use crate::oauth::pkce::is_sha256_base64url;
use axum::http::{HeaderMap, HeaderValue, Method};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use sha2::Sha256;

#[derive(Clone)]
pub(crate) struct RecoveryProof {
    token: String,
}
impl RecoveryProof {
    pub(crate) fn token(&self) -> &str {
        &self.token
    }
}

impl BrowserSecurity {
    fn recovery_name(&self) -> &'static str {
        if self.secure {
            "__Host-nvbes-recovery"
        } else {
            "nvbes-dev-recovery"
        }
    }
    pub(crate) fn recovery_cookie(&self, token: &str) -> Result<HeaderValue, BrowserError> {
        if !is_sha256_base64url(token) {
            return Err(BrowserError::Forbidden);
        }
        Ok(self.cookie(self.recovery_name(), token, 300))
    }
    pub(crate) fn clear_recovery_cookie(&self) -> HeaderValue {
        self.cookie(self.recovery_name(), "", 0)
    }
    pub(crate) fn recovery_csrf_token(
        &self,
        token: &str,
        browser: &str,
    ) -> Result<String, BrowserError> {
        Ok(URL_SAFE_NO_PAD.encode(self.recovery_mac(token, browser)?.finalize().into_bytes()))
    }
    fn recovery_mac(&self, token: &str, browser: &str) -> Result<Hmac<Sha256>, BrowserError> {
        if !is_sha256_base64url(token) || !is_sha256_base64url(browser) {
            return Err(BrowserError::Forbidden);
        }
        let mut mac = Hmac::<Sha256>::new_from_slice(token.as_bytes())
            .map_err(|_| BrowserError::Forbidden)?;
        mac.update(b"nvbes.identity.recovery-csrf.v1\0");
        mac.update(self.origin.as_bytes());
        mac.update(&[0]);
        mac.update(browser.as_bytes());
        Ok(mac)
    }
    fn verify_recovery(
        &self,
        method: &Method,
        headers: &HeaderMap,
    ) -> Result<RecoveryProof, BrowserError> {
        let browser = self.verify_mutation(method, headers)?;
        let token = read_cookie(headers, self.recovery_name())?.ok_or(BrowserError::Forbidden)?;
        let signature = URL_SAFE_NO_PAD
            .decode(&browser.csrf_token)
            .map_err(|_| BrowserError::Forbidden)?;
        self.recovery_mac(&token, &browser.browser_token)?
            .verify_slice(&signature)
            .map_err(|_| BrowserError::Forbidden)?;
        Ok(RecoveryProof { token })
    }
}

pub(crate) async fn protect_recovery_mutation(
    axum::extract::State(security): axum::extract::State<BrowserSecurity>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let mut response = match security.verify_recovery(request.method(), request.headers()) {
        Ok(proof) => {
            request.extensions_mut().insert(proof);
            next.run(request).await
        }
        Err(_) => (
            axum::http::StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({"error":"invalid_browser_request"})),
        )
            .into_response(),
    };
    response.headers_mut().extend(security.response_headers());
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oauth::store::random_secret;
    #[test]
    fn recovery_cookie_and_csrf_are_distinct_and_bound_to_browser_and_origin() {
        let security = BrowserSecurity::new("https://identity.example", false).unwrap();
        let token = random_secret();
        let browser = random_secret();
        let csrf = security.recovery_csrf_token(&token, &browser).unwrap();
        let header = security.recovery_cookie(&token).unwrap();
        let value = header.to_str().unwrap();
        assert!(value.starts_with("__Host-nvbes-recovery="));
        for property in ["HttpOnly", "Secure", "Path=/", "Max-Age=300"] {
            assert!(value.contains(property));
        }
        assert!(!value.contains("Domain="));
        let mut headers = HeaderMap::new();
        headers.insert(
            "origin",
            HeaderValue::from_static("https://identity.example"),
        );
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert(
            "cookie",
            HeaderValue::from_str(&format!(
                "__Host-nvbes-browser={browser}; __Host-nvbes-recovery={token}"
            ))
            .unwrap(),
        );
        headers.insert("x-csrf-token", HeaderValue::from_str(&csrf).unwrap());
        assert!(security.verify_recovery(&Method::POST, &headers).is_ok());
        assert!(
            security
                .verify_session_csrf(&security.verify_mutation(&Method::POST, &headers).unwrap())
                .is_err()
        );
        for wrong in [
            security.session_csrf_token(&token, &browser).unwrap(),
            security
                .recovery_csrf_token(&token, &random_secret())
                .unwrap(),
            BrowserSecurity::new("https://other.example", false)
                .unwrap()
                .recovery_csrf_token(&token, &browser)
                .unwrap(),
        ] {
            headers.insert("x-csrf-token", HeaderValue::from_str(&wrong).unwrap());
            assert!(security.verify_recovery(&Method::POST, &headers).is_err());
        }
        headers.insert("x-csrf-token", HeaderValue::from_str(&csrf).unwrap());
        headers.append(
            "cookie",
            HeaderValue::from_str(&format!("__Host-nvbes-recovery={token}")).unwrap(),
        );
        assert!(security.verify_recovery(&Method::POST, &headers).is_err());
    }
}
