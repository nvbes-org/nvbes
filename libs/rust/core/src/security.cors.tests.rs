use super::{cors_allowed_origins, cors_layer, expand_loopback_aliases, is_loopback_host};
use crate::config::AppConfig;

#[test]
fn expand_loopback_aliases_adds_localhost_variants() {
    let expanded = expand_loopback_aliases(&["http://127.0.0.1:5173".to_string()]);
    assert!(expanded.contains(&"http://127.0.0.1:5173".to_string()));
    assert!(expanded.contains(&"http://localhost:5173".to_string()));
    assert!(expanded.contains(&"http://[::1]:5173".to_string()));
}

#[test]
fn expand_loopback_aliases_skips_empty_origins() {
    assert_eq!(
        expand_loopback_aliases(&[String::new()]),
        Vec::<String>::new()
    );
}

#[test]
fn is_loopback_host_recognizes_common_hosts() {
    assert!(is_loopback_host("localhost"));
    assert!(is_loopback_host("127.0.0.1"));
    assert!(is_loopback_host("::1"));
    assert!(!is_loopback_host("example.com"));
}

#[test]
fn cors_allowed_origins_include_config_and_additional_entries() {
    let config = AppConfig {
        web_base_url: "https://app.example".into(),
        api_base_url: "https://api.example".into(),
        staging_web_base_url: Some("https://staging.example".into()),
        staging_api_base_url: Some("https://staging-api.example".into()),
        additional_cors_origins: vec!["https://extra.example".into()],
        ..AppConfig::default()
    };
    let origins: Vec<String> = cors_allowed_origins(&config)
        .into_iter()
        .filter_map(|value| value.to_str().ok().map(str::to_owned))
        .collect();
    assert!(origins.contains(&"https://app.example".to_string()));
    assert!(origins.contains(&"https://extra.example".to_string()));
    assert!(origins.contains(&"https://staging-api.example".to_string()));
}

#[test]
fn cors_layer_builds_with_default_config() {
    let _layer = cors_layer(&AppConfig::default());
}
