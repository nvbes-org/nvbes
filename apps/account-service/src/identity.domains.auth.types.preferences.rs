use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserPreferences {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub skip_password: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserNotifications {
    #[serde(default = "default_true")]
    pub email: bool,
    #[serde(default = "default_true")]
    pub push: bool,
    #[serde(default = "default_true")]
    pub in_app: bool,
    #[serde(default)]
    pub marketing_email: bool,
}

pub fn derive_display_name(
    firstname: Option<&str>,
    lastname: Option<&str>,
    username: Option<&str>,
) -> String {
    let firstname = firstname.unwrap_or("").trim();
    let lastname = lastname.unwrap_or("").trim();

    if !firstname.is_empty() && !lastname.is_empty() {
        format!("{firstname} {lastname}")
    } else {
        username
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or_default()
            .to_string()
    }
}

fn default_theme() -> String {
    "system".to_string()
}

fn default_language() -> String {
    "en".to_string()
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::derive_display_name;

    #[test]
    fn derive_display_name_uses_firstname_and_lastname_when_both_present() {
        assert_eq!(
            derive_display_name(Some("Rayane"), Some("Guemmoud"), Some("shaynlink")),
            "Rayane Guemmoud"
        );
    }

    #[test]
    fn derive_display_name_falls_back_to_username_when_profile_name_is_incomplete() {
        assert_eq!(
            derive_display_name(Some("Rayane"), None, Some("shaynlink")),
            "shaynlink"
        );
    }
}
