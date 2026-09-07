use axum::http::{HeaderMap, HeaderValue, Method, header};
use reqwest::Url;

use crate::oauth::{pkce::is_sha256_base64url, store::random_secret};

#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    #[error("invalid Identity browser origin")]
    Configuration,
    #[error("invalid browser request")]
    Forbidden,
}

#[derive(Clone)]
pub struct BrowserSecurity {
    origin: String,
    secure: bool,
}

/// Constructed only after checking HTTP method, Origin, cookies and CSRF shape.
/// The interaction store must additionally compare both hashed secrets.
#[derive(Clone)]
pub struct BrowserProof {
    pub(crate) browser_token: String,
    pub(crate) csrf_token: String,
    pub(crate) session_token: Option<String>,
}

/// Apply to mutation routes before extractors parse the request body. Successful
/// handlers receive evidence through Extension<BrowserProof>, never raw headers.
pub async fn protect_mutation(
    axum::extract::State(security): axum::extract::State<BrowserSecurity>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let mut response = match security.verify_mutation(request.method(), request.headers()) {
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

pub struct BrowserCookie {
    pub token: String,
    pub header: HeaderValue,
}

impl BrowserSecurity {
    pub fn new(origin: &str, development: bool) -> Result<Self, BrowserError> {
        let url = Url::parse(origin).map_err(|_| BrowserError::Configuration)?;
        let secure = url.scheme() == "https";
        let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if !(secure || development && url.scheme() == "http" && loopback)
            || origin.trim_end_matches('/') != url.origin().ascii_serialization()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(BrowserError::Configuration);
        }
        Ok(Self {
            origin: url.origin().ascii_serialization(),
            secure,
        })
    }

    pub fn browser_cookie(&self) -> BrowserCookie {
        let token = random_secret();
        let header = self.cookie(self.browser_name(), &token, 3600);
        BrowserCookie { token, header }
    }

    pub fn session_cookie(&self, token: &str) -> Result<HeaderValue, BrowserError> {
        if !is_sha256_base64url(token) {
            return Err(BrowserError::Forbidden);
        }
        Ok(self.cookie(self.session_name(), token, 3600))
    }

    pub fn clear_session_cookie(&self) -> HeaderValue {
        self.cookie(self.session_name(), "", 0)
    }

    pub fn browser_token(&self, headers: &HeaderMap) -> Result<Option<String>, BrowserError> {
        read_cookie(headers, self.browser_name())
    }

    pub fn session_token(&self, headers: &HeaderMap) -> Result<Option<String>, BrowserError> {
        read_cookie(headers, self.session_name())
    }

    /// Hosted interaction mutations use JSON and a header CSRF token. Cross-site
    /// OAuth top-level navigation is handled separately by the authorization GET.
    pub fn verify_mutation(
        &self,
        method: &Method,
        headers: &HeaderMap,
    ) -> Result<BrowserProof, BrowserError> {
        if method != Method::POST
            || single_header(headers, "origin")? != Some(self.origin.as_str())
            || single_header(headers, "sec-fetch-site")?.is_some_and(|v| v != "same-origin")
            || single_header(headers, "sec-fetch-mode")?
                .is_some_and(|v| !matches!(v, "cors" | "same-origin"))
            || single_header(headers, "sec-fetch-dest")?.is_some_and(|v| v != "empty")
        {
            return Err(BrowserError::Forbidden);
        }
        let content_type =
            single_header(headers, "content-type")?.ok_or(BrowserError::Forbidden)?;
        if !matches!(
            content_type.to_ascii_lowercase().as_str(),
            "application/json" | "application/json; charset=utf-8"
        ) {
            return Err(BrowserError::Forbidden);
        }
        let csrf = single_header(headers, "x-csrf-token")?.ok_or(BrowserError::Forbidden)?;
        if !is_sha256_base64url(csrf) {
            return Err(BrowserError::Forbidden);
        }
        Ok(BrowserProof {
            browser_token: self
                .browser_token(headers)?
                .ok_or(BrowserError::Forbidden)?,
            csrf_token: csrf.to_owned(),
            session_token: self.session_token(headers)?,
        })
    }

    pub fn response_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in [
            ("cache-control", "no-store"),
            ("pragma", "no-cache"),
            ("referrer-policy", "no-referrer"),
            ("x-content-type-options", "nosniff"),
            ("x-frame-options", "DENY"),
            (
                "content-security-policy",
                "default-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
            ),
        ] {
            headers.insert(name, HeaderValue::from_static(value));
        }
        if self.secure {
            headers.insert(
                "strict-transport-security",
                HeaderValue::from_static("max-age=31536000"),
            );
        }
        headers
    }

    fn browser_name(&self) -> &'static str {
        if self.secure {
            "__Host-nvbes-browser"
        } else {
            "nvbes-dev-browser"
        }
    }

    fn session_name(&self) -> &'static str {
        if self.secure {
            "__Host-nvbes-session"
        } else {
            "nvbes-dev-session"
        }
    }

    fn cookie(&self, name: &str, value: &str, max_age: u32) -> HeaderValue {
        let secure = if self.secure { "; Secure" } else { "" };
        HeaderValue::from_str(&format!(
            "{name}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}"
        ))
        .expect("cookie names and canonical base64url values are header safe")
    }
}

fn single_header<'a>(headers: &'a HeaderMap, name: &str) -> Result<Option<&'a str>, BrowserError> {
    let mut values = headers.get_all(name).iter();
    let value = values
        .next()
        .map(|h| h.to_str().map_err(|_| BrowserError::Forbidden))
        .transpose()?;
    if values.next().is_some() {
        return Err(BrowserError::Forbidden);
    }
    Ok(value)
}

fn read_cookie(headers: &HeaderMap, name: &str) -> Result<Option<String>, BrowserError> {
    let mut found = None;
    let mut bytes = 0;
    for header in headers.get_all(header::COOKIE) {
        bytes += header.len();
        if bytes > 8192 {
            return Err(BrowserError::Forbidden);
        }
        for pair in header
            .to_str()
            .map_err(|_| BrowserError::Forbidden)?
            .split(';')
        {
            let Some((key, value)) = pair.trim().split_once('=') else {
                continue;
            };
            if key == name {
                if found.is_some() || !is_sha256_base64url(value) {
                    return Err(BrowserError::Forbidden);
                }
                found = Some(value.to_owned());
            }
        }
    }
    Ok(found)
}

#[cfg(test)]
#[path = "identity.browser.tests.rs"]
mod tests;
