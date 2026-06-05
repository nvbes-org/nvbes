use axum::http::{HeaderMap, HeaderName, HeaderValue};
use chrono::{DateTime, Utc};

pub static SUNSET: HeaderName = HeaderName::from_static("sunset");
pub static DEPRECATION: HeaderName = HeaderName::from_static("deprecation");

#[derive(Debug, Clone)]
pub struct SunsetDeprecation {
    pub sunset: Option<DateTime<Utc>>,
    pub deprecation: bool,
}

impl SunsetDeprecation {
    pub fn new(sunset: DateTime<Utc>, deprecation: bool) -> Self {
        Self {
            sunset: Some(sunset),
            deprecation,
        }
    }

    pub fn deprecated_only() -> Self {
        Self {
            sunset: None,
            deprecation: true,
        }
    }
}

fn format_http_date(dt: &DateTime<Utc>) -> String {
    dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

/// Insert `Sunset` and `Deprecation` headers (RFC 8594) into a response's HeaderMap.
///
/// Use this inside a [`.layer(axum::middleware::from_fn(...))`] closure to attach
/// sunset/deprecation information to route responses:
///
/// ```ignore
/// .layer(axum::middleware::from_fn(move |_req, next: axum::middleware::Next| async move {
///     let mut response = next.run(_req).await;
///     nvbes_core::http::deprecation::insert_deprecation_headers(
///         response.headers_mut(),
///         &nvbes_core::http::deprecation::SunsetDeprecation::new(
///             chrono::Utc.with_ymd_and_hms(2026, 12, 31, 23, 59, 59).unwrap(),
///             true,
///         ),
///     );
///     response
/// }))
/// ```
pub fn insert_deprecation_headers(headers: &mut HeaderMap, config: &SunsetDeprecation) {
    if let Some(ref sunset) = config.sunset {
        let val = format_http_date(sunset);
        if let Ok(v) = HeaderValue::from_str(&val) {
            headers.insert(&SUNSET, v);
        }
    }
    if config.deprecation {
        headers.insert(&DEPRECATION, HeaderValue::from_static("true"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_both_headers() {
        let mut headers = HeaderMap::new();
        let sunset = Utc::now();
        let config = SunsetDeprecation::new(sunset, true);

        insert_deprecation_headers(&mut headers, &config);

        assert!(headers.contains_key(&SUNSET));
        assert_eq!(headers.get(&DEPRECATION).unwrap(), "true");
    }

    #[test]
    fn sets_deprecation_only() {
        let mut headers = HeaderMap::new();
        let config = SunsetDeprecation::deprecated_only();

        insert_deprecation_headers(&mut headers, &config);

        assert!(!headers.contains_key(&SUNSET));
        assert_eq!(headers.get(&DEPRECATION).unwrap(), "true");
    }

    #[test]
    fn sets_sunset_only() {
        let mut headers = HeaderMap::new();
        let sunset = Utc::now();
        let config = SunsetDeprecation {
            sunset: Some(sunset),
            deprecation: false,
        };

        insert_deprecation_headers(&mut headers, &config);

        assert!(headers.contains_key(&SUNSET));
        assert!(!headers.contains_key(&DEPRECATION));
    }

    #[test]
    fn sets_no_headers_when_none() {
        let mut headers = HeaderMap::new();
        let config = SunsetDeprecation {
            sunset: None,
            deprecation: false,
        };

        insert_deprecation_headers(&mut headers, &config);

        assert!(!headers.contains_key(&SUNSET));
        assert!(!headers.contains_key(&DEPRECATION));
    }

    #[test]
    fn sunset_header_is_http_date_format() {
        let mut headers = HeaderMap::new();
        let sunset = Utc::now();
        let config = SunsetDeprecation::new(sunset, true);

        insert_deprecation_headers(&mut headers, &config);

        let value = headers.get(&SUNSET).unwrap().to_str().unwrap();
        assert!(
            value.ends_with(" GMT"),
            "Sunset header should end with ' GMT', got: {value}"
        );
    }
}
