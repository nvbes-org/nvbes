use rand::Rng;

pub(super) fn initial_score(amr: &[String], acr: &str) -> i16 {
    20 + assurance_increment(amr, acr) + 5
}

pub(super) fn updated_score(
    previous: i16,
    previous_level: &str,
    profile_match: Option<bool>,
    amr: &[String],
    acr: &str,
) -> i16 {
    let profile_delta = match profile_match {
        Some(true) => 5,
        Some(false) => -25,
        None => 0,
    };
    let score = (previous + assurance_increment(amr, acr) + profile_delta).clamp(0, 100);
    let strong_authentication = acr == "aal2" || amr.iter().any(|method| method == "webauthn");

    if strong_authentication || (previous_level == "trusted" && profile_match != Some(false)) {
        score
    } else {
        score.min(60)
    }
}

pub(super) fn trust_level(score: i16) -> &'static str {
    if score >= 70 {
        "trusted"
    } else if score >= 40 {
        "recognized"
    } else {
        "unknown"
    }
}

pub(super) fn valid_token(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn generate_installation_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill(&mut bytes);
    hex::encode(bytes)
}

pub(super) fn display_name(user_agent: Option<&str>) -> String {
    user_agent_family(user_agent).unwrap_or_else(|| "unknown device".to_string())
}

pub(super) fn user_agent_family(user_agent: Option<&str>) -> Option<String> {
    let value = user_agent?.to_ascii_lowercase();
    let browser = if value.contains("firefox/") {
        "Firefox"
    } else if value.contains("edg/") {
        "Edge"
    } else if value.contains("chrome/") || value.contains("chromium/") {
        "Chromium"
    } else if value.contains("safari/") {
        "Safari"
    } else {
        "Other"
    };
    let platform = if value.contains("windows") {
        "Windows"
    } else if value.contains("android") {
        "Android"
    } else if value.contains("iphone") || value.contains("ipad") {
        "iOS"
    } else if value.contains("mac os") || value.contains("macintosh") {
        "macOS"
    } else if value.contains("linux") {
        "Linux"
    } else {
        "Unknown"
    };
    Some(format!("{browser} on {platform}"))
}

fn assurance_increment(amr: &[String], acr: &str) -> i16 {
    if amr.iter().any(|method| method == "webauthn") {
        30
    } else if acr == "aal2" {
        20
    } else {
        5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webauthn_reaches_recognized_without_claiming_hardware_attestation() {
        assert_eq!(initial_score(&["webauthn".to_string()], "aal2"), 55);
        assert_eq!(trust_level(55), "recognized");
    }

    #[test]
    fn profile_change_reduces_trust() {
        let score = updated_score(75, "trusted", Some(false), &["pwd".to_string()], "aal1");
        assert_eq!(score, 55);
        assert_eq!(trust_level(score), "recognized");
    }

    #[test]
    fn password_only_authentication_never_promotes_to_trusted() {
        let score = updated_score(60, "recognized", Some(true), &["pwd".to_string()], "aal1");
        assert_eq!(score, 60);
        assert_eq!(trust_level(score), "recognized");
    }

    #[test]
    fn manual_trust_survives_matching_password_login() {
        let score = updated_score(80, "trusted", Some(true), &["pwd".to_string()], "aal1");
        assert_eq!(score, 90);
        assert_eq!(trust_level(score), "trusted");
    }
}
