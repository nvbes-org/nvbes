pub(super) fn env_or_default(
    name: &str,
    value: Option<String>,
    default: &str,
    strict_mode: bool,
) -> Result<String, String> {
    match value {
        Some(value) if !value.trim().is_empty() => Ok(value.trim().to_owned()),
        _ if strict_mode => Err(format!(
            "{name} must be set when NVBES_ENV is not development"
        )),
        _ => Ok(default.to_string()),
    }
}

pub(super) fn optional_env(name: &str) -> Option<String> {
    std::env::var(name).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

pub(super) fn optional_env_any(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| optional_env(name))
}

pub(super) fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|v| v == "true" || v == "1")
        .unwrap_or(default)
}

pub(super) fn env_bool_any(names: &[&str], default: bool) -> bool {
    names
        .iter()
        .find_map(|name| std::env::var(name).ok())
        .map(|v| v == "true" || v == "1")
        .unwrap_or(default)
}

pub(super) fn parse_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
