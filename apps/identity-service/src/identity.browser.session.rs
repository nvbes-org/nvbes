use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use super::{BrowserError, BrowserProof, BrowserSecurity};
use crate::oauth::pkce::is_sha256_base64url;

#[derive(Clone)]
pub(crate) struct SessionProof {
    session_token: String,
}

impl SessionProof {
    pub(crate) fn token(&self) -> &str {
        &self.session_token
    }
}

pub(crate) async fn protect_session_mutation(
    axum::extract::State(security): axum::extract::State<BrowserSecurity>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let verified = security
        .verify_mutation(request.method(), request.headers())
        .and_then(|proof| {
            security
                .verify_session_csrf(&proof)
                .map(|token| SessionProof {
                    session_token: token.to_owned(),
                })
        });
    let mut response = match verified {
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

impl BrowserSecurity {
    /// Only return to the hosted UI in a no-store body. The random 256-bit
    /// HttpOnly session secret is the MAC key; it is never exposed to scripts.
    /// This token remains stable across tabs and rotates with either cookie.
    pub(crate) fn session_csrf_token(
        &self,
        session: &str,
        browser: &str,
    ) -> Result<String, BrowserError> {
        Ok(URL_SAFE_NO_PAD.encode(self.session_mac(session, browser)?.finalize().into_bytes()))
    }

    /// HTTP Origin/cookie checks precede this verification. Database operations
    /// must still verify session expiry, revocation and principal status.
    pub(crate) fn verify_session_csrf<'a>(
        &self,
        proof: &'a BrowserProof,
    ) -> Result<&'a str, BrowserError> {
        let session = proof
            .session_token
            .as_deref()
            .ok_or(BrowserError::Forbidden)?;
        let signature = URL_SAFE_NO_PAD
            .decode(&proof.csrf_token)
            .map_err(|_| BrowserError::Forbidden)?;
        self.session_mac(session, &proof.browser_token)?
            .verify_slice(&signature)
            .map_err(|_| BrowserError::Forbidden)?;
        Ok(session)
    }

    fn session_mac(&self, session: &str, browser: &str) -> Result<Hmac<Sha256>, BrowserError> {
        if !is_sha256_base64url(session) || !is_sha256_base64url(browser) {
            return Err(BrowserError::Forbidden);
        }
        let mut mac = Hmac::<Sha256>::new_from_slice(session.as_bytes())
            .map_err(|_| BrowserError::Forbidden)?;
        mac.update(b"nvbes.identity.session-csrf.v1\0");
        mac.update(self.origin.as_bytes());
        mac.update(&[0]);
        mac.update(browser.as_bytes());
        Ok(mac)
    }
}
