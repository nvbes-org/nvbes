use crate::config::AppConfig;
use axum::http::{HeaderName, HeaderValue, Method, Uri, header};
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer, ExposeHeaders};

const REQUEST_ID_HEADER_NAME: &str = "x-request-id";
const CSRF_TOKEN_HEADER_NAME: &str = "x-csrf-token";
const SENTRY_TRACE_HEADER_NAME: &str = "sentry-trace";
const SENTRY_BAGGAGE_HEADER_NAME: &str = "baggage";
const TRACEPARENT_HEADER_NAME: &str = "traceparent";
const TRACESTATE_HEADER_NAME: &str = "tracestate";

pub fn cors_layer(config: &AppConfig) -> CorsLayer {
    let origins = cors_allowed_origins(config);

    let allowed_origins = if origins.is_empty() {
        AllowOrigin::default()
    } else {
        AllowOrigin::list(origins)
    };

    CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods(AllowMethods::from([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ]))
        .allow_headers(AllowHeaders::from([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            HeaderName::from_static("x-requested-with"),
            HeaderName::from_static("idempotency-key"),
            HeaderName::from_static(REQUEST_ID_HEADER_NAME),
            HeaderName::from_static(CSRF_TOKEN_HEADER_NAME),
            HeaderName::from_static(SENTRY_TRACE_HEADER_NAME),
            HeaderName::from_static(SENTRY_BAGGAGE_HEADER_NAME),
            HeaderName::from_static(TRACEPARENT_HEADER_NAME),
            HeaderName::from_static(TRACESTATE_HEADER_NAME),
        ]))
        .expose_headers(ExposeHeaders::from([
            HeaderName::from_static(REQUEST_ID_HEADER_NAME),
            HeaderName::from_static(TRACEPARENT_HEADER_NAME),
            HeaderName::from_static(TRACESTATE_HEADER_NAME),
        ]))
        .allow_credentials(true)
}

fn cors_allowed_origins(config: &AppConfig) -> Vec<HeaderValue> {
    let mut origins = vec![config.web_base_url.clone(), config.api_base_url.clone()];

    if let Some(ref origin) = config.staging_web_base_url {
        origins.push(origin.clone());
    }

    if let Some(ref origin) = config.staging_api_base_url {
        origins.push(origin.clone());
    }

    origins.extend(config.additional_cors_origins.iter().cloned());

    expand_loopback_aliases(&origins)
        .into_iter()
        .filter_map(|origin| HeaderValue::from_str(&origin).ok())
        .collect()
}

fn expand_loopback_aliases(origins: &[String]) -> Vec<String> {
    let mut expanded = Vec::with_capacity(origins.len() * 3);
    for origin in origins {
        if origin.is_empty() {
            continue;
        }

        expanded.push(origin.clone());

        if let Some(aliases) = loopback_aliases(origin) {
            expanded.extend(aliases);
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

struct OriginParts {
    scheme: String,
    host: String,
    port: Option<u16>,
}

fn origin_parts(uri: &Uri) -> Option<OriginParts> {
    let scheme = uri.scheme_str()?.to_ascii_lowercase();
    let authority = uri.authority()?;
    Some(OriginParts {
        scheme,
        host: authority.host().to_ascii_lowercase(),
        port: authority.port_u16(),
    })
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}
