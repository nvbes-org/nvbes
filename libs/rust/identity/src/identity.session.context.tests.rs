use super::*;
use axum::http::HeaderValue;

const FIREFOX_LINUX: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:140.0) Gecko/20100101 Firefox/140.0";
const FIREFOX_LINUX_NEXT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:141.0) Gecko/20100101 Firefox/141.0";
const CHROME_WINDOWS: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";
const EDGE_WINDOWS: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36 Edg/141.0.0.0";
const SAFARI_IPHONE: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";

fn guard() -> SessionContextGuard {
    let limiter = RateLimiter::new([9; 32]).unwrap();
    let browser = BrowserSecurity::new("http://localhost:3000", true).unwrap();
    let db = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused@127.0.0.1:1/unused")
        .unwrap();
    SessionContextGuard::new(db, browser, &limiter).unwrap()
}

fn headers(user_agent: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        HeaderValue::from_str(user_agent).unwrap(),
    );
    headers
}

#[test]
fn families_ignore_versions_and_tell_browsers_apart() {
    assert_eq!(environment_family(FIREFOX_LINUX), "firefox/linux");
    assert_eq!(environment_family(FIREFOX_LINUX_NEXT), "firefox/linux");
    assert_eq!(environment_family(CHROME_WINDOWS), "chromium/windows");
    assert_eq!(environment_family(EDGE_WINDOWS), "edge/windows");
    assert_eq!(environment_family(SAFARI_IPHONE), "safari/ios");
    assert_eq!(environment_family(""), "other/other");
}

#[test]
fn prefixes_drop_the_host_part() {
    assert_eq!(
        network_prefix(Some("203.0.113.42".parse().unwrap())),
        "203.0.113.0/24"
    );
    assert_eq!(
        network_prefix(Some("::ffff:203.0.113.42".parse().unwrap())),
        "203.0.113.0/24"
    );
    assert_eq!(
        network_prefix(Some("2001:db8:abcd:12::1".parse().unwrap())),
        "2001:db8:abcd::/48"
    );
    assert_eq!(network_prefix(None), "unknown");
}

#[tokio::test]
async fn fingerprint_is_stable_inside_one_environment() {
    let guard = guard();
    let first = guard.fingerprint(
        &headers(FIREFOX_LINUX),
        Some("203.0.113.42".parse().unwrap()),
    );
    let after_update_and_dhcp = guard.fingerprint(
        &headers(FIREFOX_LINUX_NEXT),
        Some("203.0.113.77".parse().unwrap()),
    );
    assert_eq!(first, after_update_and_dhcp);
}

#[tokio::test]
async fn fingerprint_changes_with_browser_or_network() {
    let guard = guard();
    let base = guard.fingerprint(
        &headers(FIREFOX_LINUX),
        Some("203.0.113.42".parse().unwrap()),
    );
    let other_browser = guard.fingerprint(
        &headers(CHROME_WINDOWS),
        Some("203.0.113.42".parse().unwrap()),
    );
    let other_network = guard.fingerprint(
        &headers(FIREFOX_LINUX),
        Some("198.51.100.42".parse().unwrap()),
    );
    assert_ne!(base, other_browser);
    assert_ne!(base, other_network);
}

#[tokio::test]
async fn fingerprint_is_keyed() {
    let browser = BrowserSecurity::new("http://localhost:3000", true).unwrap();
    let db = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused@127.0.0.1:1/unused")
        .unwrap();
    let other = SessionContextGuard::new(db, browser, &RateLimiter::new([8; 32]).unwrap()).unwrap();
    let peer = Some("203.0.113.42".parse().unwrap());
    assert_ne!(
        guard().fingerprint(&headers(FIREFOX_LINUX), peer),
        other.fingerprint(&headers(FIREFOX_LINUX), peer)
    );
}

#[test]
fn issued_session_token_reads_only_a_set_session_cookie() {
    let browser = BrowserSecurity::new("http://localhost:3000", true).unwrap();
    let token = crate::oauth::store::random_secret();
    let mut headers = HeaderMap::new();
    headers.append(header::SET_COOKIE, browser.session_cookie(&token).unwrap());
    assert_eq!(browser.issued_session_token(&headers), Some(token));

    let mut cleared = HeaderMap::new();
    cleared.append(header::SET_COOKIE, browser.clear_session_cookie());
    assert_eq!(browser.issued_session_token(&cleared), None);
    assert_eq!(browser.issued_session_token(&HeaderMap::new()), None);
}
