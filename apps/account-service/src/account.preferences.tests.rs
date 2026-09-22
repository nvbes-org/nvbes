use crate::error::AccountError;

use super::{UpdatePreferences, validate_language, validate_theme};

#[test]
fn theme_validation_accepts_supported_values_only() {
    assert!(validate_theme("system").is_ok());
    assert!(validate_theme("light").is_ok());
    assert!(validate_theme("dark").is_ok());
    assert!(matches!(
        validate_theme("neon"),
        Err(AccountError::Invalid("unsupported theme"))
    ));
}

#[test]
fn language_validation_accepts_fr_and_en_only() {
    assert!(validate_language("fr").is_ok());
    assert!(validate_language("en").is_ok());
    assert!(matches!(
        validate_language("de"),
        Err(AccountError::Invalid("unsupported language"))
    ));
}

#[test]
fn update_preferences_struct_holds_validated_fields() {
    let input = UpdatePreferences {
        theme: "dark".into(),
        language: "en".into(),
    };
    assert!(validate_theme(&input.theme).is_ok());
    assert!(validate_language(&input.language).is_ok());
}
