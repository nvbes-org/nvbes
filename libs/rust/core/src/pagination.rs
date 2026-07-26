use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;
use uuid::Uuid;

const CURSOR_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeysetCursor {
    pub created_at: DateTime<Utc>,
    pub id: Uuid,
}

impl KeysetCursor {
    pub fn encode(self) -> Result<String, CursorError> {
        encode_cursor(&self)
    }

    pub fn decode(value: &str) -> Result<Self, CursorError> {
        decode_cursor(value)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CursorEnvelope<T> {
    version: u8,
    payload: T,
}

#[derive(Debug, Error)]
#[error("invalid pagination cursor")]
pub struct CursorError;

pub fn encode_cursor<T: Serialize>(payload: &T) -> Result<String, CursorError> {
    let envelope = CursorEnvelope {
        version: CURSOR_VERSION,
        payload,
    };
    let json = serde_json::to_vec(&envelope).map_err(|_| CursorError)?;
    Ok(URL_SAFE_NO_PAD.encode(json))
}

pub fn decode_cursor<T: DeserializeOwned>(value: &str) -> Result<T, CursorError> {
    let json = URL_SAFE_NO_PAD.decode(value).map_err(|_| CursorError)?;
    let envelope: CursorEnvelope<T> = serde_json::from_slice(&json).map_err(|_| CursorError)?;
    if envelope.version != CURSOR_VERSION {
        return Err(CursorError);
    }
    Ok(envelope.payload)
}

#[derive(Debug, PartialEq, Eq)]
pub struct Page<T, C = String> {
    pub items: Vec<T>,
    pub next_cursor: Option<C>,
    pub has_more: bool,
}

pub fn page_from_rows<T, C>(
    mut rows: Vec<T>,
    limit: usize,
    cursor_for: impl FnOnce(&T) -> C,
) -> Page<T, C> {
    let has_more = rows.len() > limit;
    rows.truncate(limit);
    let next_cursor = if has_more {
        rows.last().map(cursor_for)
    } else {
        None
    };
    Page {
        items: rows,
        next_cursor,
        has_more,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use super::{KeysetCursor, decode_cursor, encode_cursor, page_from_rows};

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct NamedCursor {
        normalized_name: String,
        id: Uuid,
    }

    #[test]
    fn generic_cursor_codec_round_trips_domain_payloads() {
        let cursor = NamedCursor {
            normalized_name: "documents".to_string(),
            id: Uuid::from_u128(42),
        };

        let encoded = encode_cursor(&cursor).unwrap();
        let decoded: NamedCursor = decode_cursor(&encoded).unwrap();

        assert_eq!(decoded, cursor);
    }

    #[test]
    fn cursor_round_trips_as_an_opaque_versioned_token() {
        let cursor = KeysetCursor {
            created_at: Utc.with_ymd_and_hms(2026, 7, 21, 10, 30, 0).unwrap(),
            id: Uuid::parse_str("018f0000-0000-7000-8000-000000000001").unwrap(),
        };

        let encoded = cursor.encode().unwrap();
        let decoded = KeysetCursor::decode(&encoded).unwrap();

        assert_eq!(decoded, cursor);
        assert!(!encoded.contains("2026-07-21"));
    }

    #[test]
    fn cursor_rejects_an_unknown_version() {
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            r#"{"version":2,"created_at":"2026-07-21T10:30:00Z","id":"018f0000-0000-7000-8000-000000000001"}"#,
        );

        assert!(KeysetCursor::decode(&encoded).is_err());
    }

    #[test]
    fn page_uses_the_extra_row_only_to_report_more_results() {
        let page = page_from_rows(vec![1, 2, 3], 2, |value| value.to_string());

        assert_eq!(page.items, vec![1, 2]);
        assert!(page.has_more);
        assert_eq!(page.next_cursor.as_deref(), Some("2"));
    }

    #[test]
    fn final_page_has_no_cursor() {
        let page = page_from_rows(vec![1, 2], 2, |value| value.to_string());

        assert_eq!(page.items, vec![1, 2]);
        assert!(!page.has_more);
        assert_eq!(page.next_cursor, None);
    }
}
