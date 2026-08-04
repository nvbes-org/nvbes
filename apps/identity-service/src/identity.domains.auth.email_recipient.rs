use super::types::DEFAULT_DISPLAY_NAME;

pub fn recipient_name(display_name: Option<&str>) -> String {
    display_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_DISPLAY_NAME)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::recipient_name;

    #[test]
    fn uses_the_canonical_display_name() {
        assert_eq!(recipient_name(Some(" Ada Lovelace ")), "Ada Lovelace");
        assert_eq!(recipient_name(Some("User")), "User");
    }

    #[test]
    fn falls_back_to_the_default_display_name() {
        assert_eq!(recipient_name(None), "User");
        assert_eq!(recipient_name(Some("  ")), "User");
    }
}
