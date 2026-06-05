use axum::{
    extract::State,
    http::Uri,
    http::{HeaderMap, Method, Request, header},
    middleware::Next,
    response::Response,
};

use crate::{app::AppState, http::error::AppError};

const ORIGIN_SKIP_PATHS: &[&str] = &[
    "/oauth/token",
    "/oauth/introspect",
    "/oauth/revoke",
    "/oauth/par",
];

pub async fn origin_guard(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, AppError> {
    if !is_mutating_method(request.method()) {
        return Ok(next.run(request).await);
    }

    if ORIGIN_SKIP_PATHS.contains(&request.uri().path()) {
        return Ok(next.run(request).await);
    }

    if headers.get(header::AUTHORIZATION).is_some() {
        return Ok(next.run(request).await);
    }

    let allowed_origins = allowed_origins(&state);
    if allowed_origins.is_empty() {
        return Ok(next.run(request).await);
    }

    if let Some(origin) = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    {
        if allowed_origins
            .iter()
            .any(|allowed| same_origin(origin, allowed))
        {
            return Ok(next.run(request).await);
        }
    }

    if let Some(referer) = headers
        .get(header::REFERER)
        .and_then(|value| value.to_str().ok())
    {
        if allowed_origins
            .iter()
            .any(|allowed| same_origin(referer, allowed))
        {
            return Ok(next.run(request).await);
        }
    }

    Err(AppError::forbidden(
        "invalid_origin",
        "Request origin is not allowed.",
    ))
}

fn is_mutating_method(method: &Method) -> bool {
    matches!(
        method,
        &Method::POST | &Method::PUT | &Method::PATCH | &Method::DELETE
    )
}

fn allowed_origins(state: &AppState) -> Vec<String> {
    let mut origins = vec![
        state.config.web_base_url.clone(),
        state.config.api_base_url.clone(),
    ];

    if let Some(origin) = &state.config.staging_web_base_url {
        origins.push(origin.clone());
    }

    if let Some(origin) = &state.config.staging_api_base_url {
        origins.push(origin.clone());
    }

    let mut expanded = Vec::with_capacity(origins.len() * 3);
    for origin in origins {
        if origin.is_empty() {
            continue;
        }

        expanded.push(origin.clone());

        if let Some(loopback_aliases) = loopback_aliases(&origin) {
            expanded.extend(loopback_aliases);
        }
    }

    expanded
}

fn loopback_aliases(origin: &str) -> Option<Vec<String>> {
    let uri = origin.parse::<Uri>().ok()?;
    let parts = origin_parts(&uri)?;
    if !is_loopback_host(&parts.host) {
        return None;
    }

    let port = parts
        .port
        .map(|port| format!(":{}", port))
        .unwrap_or_default();
    let scheme = parts.scheme;
    Some(vec![
        format!("{scheme}://localhost{port}"),
        format!("{scheme}://127.0.0.1{port}"),
        format!("{scheme}://[::1]{port}"),
    ])
}

fn same_origin(candidate: &str, allowed: &str) -> bool {
    let candidate = match candidate.parse::<Uri>() {
        Ok(uri) => uri,
        Err(_) => return false,
    };
    let allowed = match allowed.parse::<Uri>() {
        Ok(uri) => uri,
        Err(_) => return false,
    };

    origin_parts(&candidate).is_some_and(|candidate_origin| {
        origin_parts(&allowed).is_some_and(|allowed_origin| {
            candidate_origin == allowed_origin
                || same_loopback_origin(&candidate_origin, &allowed_origin)
        })
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UriOrigin {
    scheme: String,
    host: String,
    port: Option<u16>,
}

fn same_loopback_origin(candidate: &UriOrigin, allowed: &UriOrigin) -> bool {
    candidate.scheme == allowed.scheme
        && candidate.port == allowed.port
        && is_loopback_host(&candidate.host)
        && is_loopback_host(&allowed.host)
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

fn origin_parts(uri: &Uri) -> Option<UriOrigin> {
    let scheme = uri.scheme_str()?.to_ascii_lowercase();
    let authority = uri.authority()?;
    Some(UriOrigin {
        scheme,
        host: authority.host().to_ascii_lowercase(),
        port: authority.port_u16(),
    })
}

#[cfg(test)]
mod tests {
    use super::same_origin;

    #[test]
    fn same_origin_rejects_prefix_matches() {
        assert!(!same_origin(
            "https://good.example.evil.com/path",
            "https://good.example"
        ));
    }

    #[test]
    fn same_origin_accepts_same_host_and_port() {
        assert!(same_origin(
            "https://good.example/path?foo=bar",
            "https://good.example"
        ));
    }

    #[test]
    fn same_origin_accepts_loopback_aliases() {
        assert!(same_origin(
            "http://127.0.0.1:3001/path",
            "http://localhost:3001"
        ));
    }

    #[test]
    fn loopback_aliases_expand_between_localhost_and_ipv4() {
        let aliases = super::loopback_aliases("http://localhost:3001")
            .expect("loopback aliases should be generated");

        assert!(
            aliases
                .iter()
                .any(|origin| origin == "http://127.0.0.1:3001")
        );
        assert!(aliases.iter().any(|origin| origin == "http://[::1]:3001"));
    }
}
