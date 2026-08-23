use proptest::prelude::*;

use super::logic::{
    max_upload_bytes, normalize_checksum, validate_mime_type, validate_object_name, validate_size,
};

proptest! {
    #[test]
    fn accepted_object_names_preserve_safe_trimmed_value(name in ".{0,512}") {
        if let Ok(value) = validate_object_name(&name) {
            prop_assert_eq!(value.as_str(), name.trim());
            prop_assert!(!value.is_empty());
            prop_assert_ne!(value.as_str(), ".");
            prop_assert_ne!(value.as_str(), "..");
            prop_assert!(!value.contains('/'));
            prop_assert!(!value.contains('\0'));
        }
    }

    #[test]
    fn accepted_upload_sizes_stay_within_cap(size in any::<i64>()) {
        match validate_size(size) {
            Ok(value) => {
                prop_assert!(value > 0);
                prop_assert!(value <= max_upload_bytes());
            }
            Err(_) => prop_assert!(size <= 0 || size > max_upload_bytes()),
        }
    }

    #[test]
    fn accepted_mime_types_are_non_empty_and_structured(mime in ".{0,512}") {
        if let Ok(value) = validate_mime_type(&mime) {
            prop_assert_eq!(value.as_str(), mime.trim());
            prop_assert!(value.contains('/'));
        }
    }

    #[test]
    fn accepted_checksums_are_normalized_hex(checksum in ".{0,512}") {
        if let Ok(Some(value)) = normalize_checksum(Some(checksum)) {
            prop_assert!(!value.is_empty());
            let is_hex = value
                .chars()
                .all(|character| character.is_ascii_hexdigit());
            prop_assert!(is_hex);
            let lowercase = value.to_ascii_lowercase();
            prop_assert_eq!(value.as_str(), lowercase.as_str());
        }
    }
}
