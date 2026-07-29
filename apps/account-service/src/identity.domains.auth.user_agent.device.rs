pub(super) fn detect_os(user_agent: &str) -> (Option<&'static str>, Option<String>) {
    let lower = user_agent.to_ascii_lowercase();

    if lower.contains("xbox") {
        return (Some("Xbox"), None);
    }
    if lower.contains("nintendo") {
        return (Some("Nintendo"), None);
    }
    if lower.contains("playstation") {
        return (Some("PlayStation"), None);
    }
    if contains_any(&lower, &["blackberry", "playbook", "bb10"]) {
        return (Some("BlackBerry"), None);
    }
    if lower.contains("windows") {
        if contains_any(&lower, &["windows phone", "wpdesktop"]) {
            return (Some("Windows Phone"), None);
        }
        if lower.contains("mobile") && !lower.contains("iemobile") {
            return (Some("Windows Mobile"), None);
        }
        let version = version_fragment_after(user_agent, "Windows NT ", false)
            .and_then(|version| windows_version(&version).map(str::to_string));
        return (Some("Windows"), version);
    }
    if contains_any(&lower, &["iphone", "ipad", "ipod"]) {
        let version = version_fragment_after(user_agent, " OS ", true)
            .or_else(|| version_fragment_after(user_agent, "CPU OS ", true));
        return (Some("iOS"), version);
    }
    if lower.contains("watch os") || lower.contains("watchos") {
        return (
            Some("watchOS"),
            version_fragment_after(user_agent, "watch os,", false),
        );
    }
    if lower.contains("android") {
        return (
            Some("Android"),
            version_fragment_after(user_agent, "Android ", true),
        );
    }
    if lower.contains("mac os x") {
        return (
            Some("Mac OS X"),
            version_fragment_after(user_agent, "Mac OS X ", true),
        );
    }
    if lower.contains("macintosh") || lower.contains(" mac ") {
        return (Some("Mac OS X"), None);
    }
    if lower.contains("cros") {
        return (Some("Chrome OS"), None);
    }
    if lower.contains("linux") || lower.contains("debian") {
        return (Some("Linux"), None);
    }
    (None, None)
}

pub(super) fn detect_device(user_agent: &str) -> Option<&'static str> {
    let lower = user_agent.to_ascii_lowercase();

    if lower.contains("nintendo") {
        return Some("Nintendo");
    }
    if lower.contains("playstation") {
        return Some("PlayStation");
    }
    if lower.contains("xbox") {
        return Some("Xbox");
    }
    if lower.contains("ouya") {
        return Some("Ouya");
    }
    if contains_any(&lower, &["windows phone", "wpdesktop"]) {
        return Some("Windows Phone");
    }
    if lower.contains("ipad") {
        return Some("iPad");
    }
    if lower.contains("ipod") {
        return Some("iPod Touch");
    }
    if lower.contains("iphone") {
        return Some("iPhone");
    }
    if lower.contains("watch os") || lower.contains("watchos") {
        return Some("Apple Watch");
    }
    if contains_any(&lower, &["blackberry", "playbook", "bb10"]) {
        return Some("BlackBerry");
    }
    if lower.contains("kobo") {
        return Some("Kobo");
    }
    if lower.contains("nokia") {
        return Some("Nokia");
    }
    if lower.contains("kindle") || lower.contains("silk/") {
        return Some("Kindle Fire");
    }
    if lower.contains("android") || lower.contains("zte") {
        if !lower.contains("mobile") || contains_any(&lower, &["pixel c", "nexus 7", "nexus 9"]) {
            return Some("Android Tablet");
        }
        return Some("Android");
    }
    if lower.contains("mobile") || lower.contains(" pda") {
        return Some("Generic mobile");
    }
    if lower.contains("tablet") && !lower.contains("tablet pc") {
        return Some("Generic tablet");
    }
    None
}

pub(super) fn detect_device_type(
    device: Option<&str>,
    browser: Option<&str>,
    os: Option<&str>,
) -> &'static str {
    match device {
        Some("iPad" | "Android Tablet" | "Kobo" | "Kindle Fire" | "Generic tablet") => "tablet",
        Some("Nintendo" | "Xbox" | "PlayStation" | "Ouya") => "console",
        Some("Apple Watch") => "wearable",
        Some(_) => "mobile",
        None if browser.is_some() || os.is_some() => "desktop",
        None => "unknown",
    }
}

fn version_fragment_after(value: &str, marker: &str, fill_three_parts: bool) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    let marker_lower = marker.to_ascii_lowercase();
    let start = lower.find(&marker_lower)? + marker.len();
    let raw: String = value[start..]
        .chars()
        .take_while(|character| {
            character.is_ascii_digit() || *character == '.' || *character == '_'
        })
        .collect();
    if raw.is_empty() {
        return None;
    }
    let mut parts: Vec<&str> = raw
        .split(['.', '_'])
        .filter(|part| !part.is_empty())
        .collect();
    if fill_three_parts {
        while parts.len() < 3 {
            parts.push("0");
        }
        parts.truncate(3);
    }
    Some(parts.join("."))
}

fn windows_version(value: &str) -> Option<&'static str> {
    match value {
        "3.51" => Some("NT 3.11"),
        "4.0" => Some("NT 4.0"),
        "5.0" => Some("2000"),
        "5.1" | "5.2" => Some("XP"),
        "6.0" => Some("Vista"),
        "6.1" => Some("7"),
        "6.2" => Some("8"),
        "6.3" => Some("8.1"),
        "6.4" | "10.0" => Some("10"),
        _ => None,
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}
