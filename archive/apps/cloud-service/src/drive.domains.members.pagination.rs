use nvbes_core::pagination::{CursorError, decode_cursor, encode_cursor};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use chrono::{DateTime, Utc};

use super::db::{InvitationRecord, MemberRecord};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct MemberListCursor {
    pub(super) role_rank: i16,
    pub(super) normalized_email: String,
    pub(super) user_id: Uuid,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct InvitationListCursor {
    pub(super) created_at: DateTime<Utc>,
    pub(super) id: Uuid,
}

impl InvitationListCursor {
    pub(super) fn from_record(record: &InvitationRecord) -> Self {
        Self {
            created_at: record.created_at,
            id: record.id,
        }
    }

    pub(super) fn encode(&self) -> Result<String, CursorError> {
        encode_cursor(self)
    }

    pub(super) fn decode(value: &str) -> Result<Self, CursorError> {
        let cursor: Self = decode_cursor(value)?;
        Ok(cursor)
    }
}

impl MemberListCursor {
    pub(super) fn from_record(record: &MemberRecord) -> Self {
        Self {
            role_rank: match record.role.as_str() {
                "owner" => 0,
                "admin" => 1,
                "security_admin" => 2,
                "billing_admin" => 3,
                "member" => 4,
                "viewer" => 5,
                _ => 6,
            },
            normalized_email: record.email.to_lowercase(),
            user_id: record.user_id,
        }
    }

    pub(super) fn encode(&self) -> Result<String, CursorError> {
        encode_cursor(self)
    }

    pub(super) fn decode(value: &str) -> Result<Self, CursorError> {
        let cursor: Self = decode_cursor(value)?;
        if !(0..=6).contains(&cursor.role_rank) || cursor.normalized_email.len() > 320 {
            return Err(CursorError);
        }
        Ok(cursor)
    }
}

pub(super) fn normalize_email_prefix(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .replace('\\', "\\\\")
                .replace('_', "\\_")
                .replace('%', "\\%")
        })
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::{MemberListCursor, normalize_email_prefix};

    #[test]
    fn member_cursor_round_trips_ordering_values() {
        let cursor = MemberListCursor {
            role_rank: 1,
            normalized_email: "admin@example.com".to_string(),
            user_id: Uuid::from_u128(42),
        };

        let encoded = cursor.encode().unwrap();

        assert_eq!(MemberListCursor::decode(&encoded).unwrap(), cursor);
    }

    #[test]
    fn email_prefix_is_normalized_and_escaped_for_like() {
        assert_eq!(
            normalize_email_prefix(Some("  Team_%\\  ".to_string())).as_deref(),
            Some("team\\_\\%\\\\")
        );
        assert_eq!(normalize_email_prefix(Some("  ".to_string())), None);
    }
}
