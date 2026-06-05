use axum::http::{HeaderMap, HeaderName, HeaderValue};

pub static PREFER_HEADER: HeaderName = HeaderName::from_static("prefer");
pub static PREFERENCE_APPLIED_HEADER: HeaderName = HeaderName::from_static("preference-applied");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreferHeader {
    pub return_preference: Option<ReturnPreference>,
    pub wait_seconds: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnPreference {
    Minimal,
    Representation,
}

pub fn parse_prefer(headers: &HeaderMap) -> Option<PreferHeader> {
    let raw = headers.get(&PREFER_HEADER)?.to_str().ok()?;

    let mut return_preference = None;
    let mut wait_seconds = None;

    for pref in raw.split(',') {
        let pref = pref.trim();
        let (token, value) = match pref.split_once('=') {
            Some((t, v)) => (t.trim(), Some(v.trim())),
            None => (pref.trim(), None),
        };

        match token.to_lowercase().as_str() {
            "return" => match value.map(|v| v.to_lowercase()).as_deref() {
                Some("minimal") => return_preference = Some(ReturnPreference::Minimal),
                Some("representation") => {
                    return_preference = Some(ReturnPreference::Representation)
                }
                _ => {}
            },
            "wait" => {
                if let Some(v) = value {
                    wait_seconds = v.parse::<u32>().ok();
                }
            }
            _ => {}
        }
    }

    if return_preference.is_none() && wait_seconds.is_none() {
        None
    } else {
        Some(PreferHeader {
            return_preference,
            wait_seconds,
        })
    }
}

pub fn preference_applied_minimal() -> (HeaderName, HeaderValue) {
    (
        PREFERENCE_APPLIED_HEADER.clone(),
        HeaderValue::from_static("return=minimal"),
    )
}

pub fn preference_applied_wait(seconds: u32) -> (HeaderName, HeaderValue) {
    (
        PREFERENCE_APPLIED_HEADER.clone(),
        HeaderValue::from_str(&format!("wait={seconds}"))
            .unwrap_or(HeaderValue::from_static("wait=0")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_return_minimal() {
        let mut headers = HeaderMap::new();
        headers.insert(&PREFER_HEADER, HeaderValue::from_static("return=minimal"));

        let parsed = parse_prefer(&headers).unwrap();
        assert_eq!(parsed.return_preference, Some(ReturnPreference::Minimal));
        assert_eq!(parsed.wait_seconds, None);
    }

    #[test]
    fn parse_return_representation() {
        let mut headers = HeaderMap::new();
        headers.insert(
            &PREFER_HEADER,
            HeaderValue::from_static("return=representation"),
        );

        let parsed = parse_prefer(&headers).unwrap();
        assert_eq!(
            parsed.return_preference,
            Some(ReturnPreference::Representation)
        );
    }

    #[test]
    fn parse_wait() {
        let mut headers = HeaderMap::new();
        headers.insert(&PREFER_HEADER, HeaderValue::from_static("wait=60"));

        let parsed = parse_prefer(&headers).unwrap();
        assert_eq!(parsed.return_preference, None);
        assert_eq!(parsed.wait_seconds, Some(60));
    }

    #[test]
    fn parse_multiple_prefs() {
        let mut headers = HeaderMap::new();
        headers.insert(
            &PREFER_HEADER,
            HeaderValue::from_static("return=minimal, wait=30"),
        );

        let parsed = parse_prefer(&headers).unwrap();
        assert_eq!(parsed.return_preference, Some(ReturnPreference::Minimal));
        assert_eq!(parsed.wait_seconds, Some(30));
    }

    #[test]
    fn parse_case_insensitive() {
        let mut headers = HeaderMap::new();
        headers.insert(&PREFER_HEADER, HeaderValue::from_static("RETURN=MINIMAL"));

        let parsed = parse_prefer(&headers).unwrap();
        assert_eq!(parsed.return_preference, Some(ReturnPreference::Minimal));
    }

    #[test]
    fn parse_invalid_values_returns_none() {
        let mut headers = HeaderMap::new();
        headers.insert(&PREFER_HEADER, HeaderValue::from_static("unknown=value"));

        assert!(parse_prefer(&headers).is_none());
    }

    #[test]
    fn parse_no_header_returns_none() {
        let headers = HeaderMap::new();
        assert!(parse_prefer(&headers).is_none());
    }

    #[test]
    fn parse_invalid_wait_returns_none() {
        let mut headers = HeaderMap::new();
        headers.insert(&PREFER_HEADER, HeaderValue::from_static("wait=abc"));

        let parsed = parse_prefer(&headers);
        assert!(parsed.is_none());
    }

    #[test]
    fn parse_wait_with_minimal_ignores_invalid_wait() {
        let mut headers = HeaderMap::new();
        headers.insert(
            &PREFER_HEADER,
            HeaderValue::from_static("return=minimal, wait=abc"),
        );

        let parsed = parse_prefer(&headers).unwrap();
        assert_eq!(parsed.return_preference, Some(ReturnPreference::Minimal));
        assert_eq!(parsed.wait_seconds, None);
    }
}
