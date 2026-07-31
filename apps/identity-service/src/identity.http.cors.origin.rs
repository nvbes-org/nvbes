use axum::http::Uri;

pub fn origin_from_redirect_uri(redirect_uri: &str) -> Option<String> {
    let uri = redirect_uri.parse::<Uri>().ok()?;
    let parts = origin_parts(&uri)?;
    let port = parts
        .port
        .map(|port| format!(":{port}"))
        .unwrap_or_default();
    Some(format!(
        "{}://{}{}",
        parts.scheme,
        origin_host(&parts.host),
        port
    ))
}

pub(super) fn expand_loopback_aliases(origins: Vec<String>) -> Vec<String> {
    let mut expanded = Vec::with_capacity(origins.len() * 3);
    for origin in origins {
        if origin.is_empty() {
            continue;
        }

        expanded.push(origin.clone());

        if let Some(aliases) = loopback_aliases(&origin) {
            expanded.extend(aliases);
        }
    }
    expanded
}

pub fn same_origin(candidate: &str, allowed: &str) -> bool {
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

fn origin_host(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') {
        return format!("[{host}]");
    }

    host.to_string()
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
