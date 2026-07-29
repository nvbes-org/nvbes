pub(super) fn detect_browser(
    user_agent: &str,
    browser_brands: Option<&str>,
) -> Option<&'static str> {
    let lower = user_agent.to_ascii_lowercase();
    let brands = browser_brands.unwrap_or_default().to_ascii_lowercase();

    if brands.contains("brave") || lower.contains("brave/") {
        return Some("Brave");
    }
    if lower.contains("gsa/") {
        return Some("Google Search App");
    }
    if lower.contains(" opr/") && lower.contains("mini") {
        return Some("Opera Mini");
    }
    if lower.contains(" opr/") || lower.contains("opera/") {
        return Some("Opera");
    }
    if contains_any(&lower, &["blackberry", "playbook", "bb10"]) {
        return Some("BlackBerry");
    }
    if contains_any(&lower, &["iemobile", "wpdesktop"]) {
        return Some("Internet Explorer Mobile");
    }
    if lower.contains("oculusbrowser/") {
        return Some("Oculus Browser");
    }
    if lower.contains("samsungbrowser/") {
        return Some("Samsung Internet");
    }
    if contains_any(&lower, &["edge/", "edg/", "edga/", "edgios/"]) {
        return Some("Microsoft Edge");
    }
    if lower.contains("vivaldi/") {
        return Some("Vivaldi");
    }
    if lower.contains("yabrowser/") {
        return Some("Yandex");
    }
    if lower.contains("whale/") {
        return Some("Whale");
    }
    if contains_any(&lower, &["duckduckgo/", "ddg/"]) {
        return Some("DuckDuckGo");
    }
    if lower.contains("fbios") {
        return Some("Facebook Mobile");
    }
    if contains_any(&lower, &["ucweb/", "ucbrowser/"]) {
        return Some("UC Browser");
    }
    if lower.contains("crios/") {
        return Some("Chrome iOS");
    }
    if contains_any(&lower, &["chrome/", "chromium/", "crmo/"]) {
        return Some("Chrome");
    }
    if lower.contains("android") && lower.contains("safari/") {
        return Some("Android Mobile");
    }
    if lower.contains("fxios/") {
        return Some("Firefox iOS");
    }
    if lower.contains("konqueror") {
        return Some("Konqueror");
    }
    if is_safari(&lower) {
        return Some(if lower.contains("mobile") {
            "Mobile Safari"
        } else {
            "Safari"
        });
    }
    if lower.contains("palemoon/") {
        return Some("Pale Moon");
    }
    if lower.contains("waterfox/") {
        return Some("Waterfox");
    }
    if lower.contains("firefox/") || lower.contains("gecko/") {
        return Some("Firefox");
    }
    if lower.contains("msie ") || lower.contains("trident/") {
        return Some("Internet Explorer");
    }
    None
}

pub(super) fn detect_browser_version(user_agent: &str, browser: &str) -> Option<f64> {
    let markers: &[&str] = match browser {
        "Brave" => &["Brave/"],
        "Google Search App" => &["GSA/"],
        "Opera Mini" | "Opera" => &["OPR/", "Opera/"],
        "BlackBerry" => &["BlackBerry ", "Version/"],
        "Internet Explorer Mobile" => &["rv:"],
        "Oculus Browser" => &["OculusBrowser/"],
        "Samsung Internet" => &["SamsungBrowser/"],
        "Microsoft Edge" => &["EdgiOS/", "EdgA/", "Edg/", "Edge/"],
        "Vivaldi" => &["Vivaldi/"],
        "Yandex" => &["YaBrowser/"],
        "Whale" => &["Whale/"],
        "DuckDuckGo" => &["DuckDuckGo/", "Ddg/"],
        "UC Browser" => &["UCBrowser/", "UCWEB/"],
        "Chrome iOS" => &["CriOS/"],
        "Chrome" => &["Chrome/", "Chromium/", "CrMo/"],
        "Android Mobile" => &["Android "],
        "Firefox iOS" => &["FxiOS/"],
        "Konqueror" => &["Konqueror/", "Konqueror:"],
        "Mobile Safari" | "Safari" => &["Version/"],
        "Pale Moon" => &["PaleMoon/"],
        "Waterfox" => &["Waterfox/"],
        "Firefox" => &["Firefox/"],
        "Internet Explorer" => &["MSIE ", "rv:"],
        _ => &[],
    };

    markers
        .iter()
        .find_map(|marker| numeric_version_after(user_agent, marker))
}

fn numeric_version_after(value: &str, marker: &str) -> Option<f64> {
    let start = value.find(marker)? + marker.len();
    let raw: String = value[start..]
        .chars()
        .take_while(|character| character.is_ascii_digit() || *character == '.')
        .collect();
    let mut parts = raw.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next().and_then(|value| value.parse::<u32>().ok());
    match minor {
        Some(minor) => format!("{major}.{minor}").parse().ok(),
        None => Some(f64::from(major)),
    }
}

fn is_safari(lower: &str) -> bool {
    lower.contains("safari/")
        && !contains_any(lower, &["chrome/", "chromium/", "android", "crios/"])
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}
