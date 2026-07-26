use nvbes_core::pagination::{CursorError, decode_cursor, encode_cursor};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::models::{StorageObjectRecord, StorageObjectType};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ObjectListCursor {
    pub(super) object_type_rank: i16,
    pub(super) normalized_name: String,
    pub(super) id: Uuid,
}

impl ObjectListCursor {
    pub(super) fn from_record(record: &StorageObjectRecord) -> Self {
        Self {
            object_type_rank: match &record.object_type {
                StorageObjectType::Folder => 0,
                StorageObjectType::File => 1,
            },
            normalized_name: record.name.to_lowercase(),
            id: record.id,
        }
    }

    pub(super) fn encode(&self) -> Result<String, CursorError> {
        encode_cursor(self)
    }

    pub(super) fn decode(value: &str) -> Result<Self, CursorError> {
        let cursor: Self = decode_cursor(value)?;
        if !(0..=1).contains(&cursor.object_type_rank) || cursor.normalized_name.len() > 255 {
            return Err(CursorError);
        }
        Ok(cursor)
    }
}

pub(super) fn normalize_name_prefix(value: Option<String>) -> Option<String> {
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

    use super::{ObjectListCursor, normalize_name_prefix};

    #[test]
    fn object_cursor_round_trips_name_ordering_values() {
        let cursor = ObjectListCursor {
            object_type_rank: 0,
            normalized_name: "documents".to_string(),
            id: Uuid::from_u128(42),
        };

        let encoded = cursor.encode().unwrap();

        assert_eq!(ObjectListCursor::decode(&encoded).unwrap(), cursor);
    }

    #[test]
    fn name_prefix_is_trimmed_lowercased_and_escaped_for_like() {
        assert_eq!(
            normalize_name_prefix(Some("  Q3_%\\  ".to_string())).as_deref(),
            Some("q3\\_\\%\\\\")
        );
        assert_eq!(normalize_name_prefix(Some("  ".to_string())), None);
    }
}
