use super::{
    invitation_email, password_change_code_email, password_reset_email, verification_email,
};
use crate::app::AppConfig;

fn mock_config() -> AppConfig {
    AppConfig {
        web_base_url: "https://nvbes.fr".to_string(),
        auth_verification_ttl_hours: 24,
        auth_password_reset_ttl_minutes: 30,
        email_from_email: Some("test@nvbes.fr".to_string()),
        ..Default::default()
    }
}

#[tokio::test]
async fn test_verification_email_substitution() {
    let config = mock_config();
    let email =
        verification_email(&config, "shayn@nvbes.fr", "Shayn", "123").expect("email should build");
    let html = email.html_body.unwrap();
    assert!(html.contains("Shayn"));
    assert!(html.contains("https://nvbes.fr/verify-result?token=123"));
    // Note: The template title in HTML is currently in English in our preview implementation
    assert!(html.contains("Verify your email"));
}

#[tokio::test]
async fn test_development_mock_email_uses_default_sender() {
    let mut config = mock_config();
    config.email_from_email = None;
    config.email_provider = "mock".to_string();
    config.environment = "development".to_string();

    let email = verification_email(&config, "shayn@nvbes.fr", "Shayn", "123")
        .expect("mock development email should build without explicit sender");

    assert_eq!(email.from.email, "dev@nvbes.local");
    assert_eq!(
        email.headers,
        vec![("Reply-To".to_string(), "dev@nvbes.local".to_string())]
    );
}

#[tokio::test]
async fn test_password_reset_email_substitution() {
    let config = mock_config();
    let email = password_reset_email(&config, "shayn@nvbes.fr", "Shayn", "abc")
        .expect("email should build");
    let html = email.html_body.unwrap();
    assert!(html.contains("Shayn"));
    assert!(html.contains("https://nvbes.fr/reset-password?token=abc"));
}

#[tokio::test]
async fn password_change_code_email_is_scoped_and_contains_expiry() {
    let config = mock_config();
    let email = password_change_code_email(&config, "shayn@nvbes.fr", "Shayn", "123456", 10)
        .expect("password change code email");
    let html = email.html_body.expect("html body");
    assert!(html.contains("123456"));
    assert!(html.contains("10 minutes"));
    assert!(html.contains("ne peut être utilisé que pour modifier votre mot de passe"));
}

#[tokio::test]
async fn test_invitation_email_substitution() {
    let config = mock_config();
    let email = invitation_email(&config, "new@nvbes.fr", "nvbes Team", "Shayn", "999")
        .expect("email should build");
    let html = email.html_body.unwrap();
    assert!(html.contains("Shayn"));
    assert!(html.contains("nvbes Team"));
    assert!(html.contains("https://nvbes.fr/join?token=999"));
}

#[tokio::test]
async fn test_html_escape_prevents_injection_in_name() {
    let config = mock_config();
    let email = verification_email(
        &config,
        "test@nvbes.fr",
        "<script>alert('xss')</script>",
        "token123",
    )
    .expect("email should build");
    let html = email.html_body.unwrap();
    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("alert(&#39;xss&#39;)"));
    assert!(html.contains("&lt;/script&gt;"));
}

#[tokio::test]
async fn test_html_escape_prevents_injection_in_invitation() {
    let config = mock_config();
    let email = invitation_email(
        &config,
        "test@nvbes.fr",
        "</p><img src=x onerror=alert(1)>",
        "Hacker</a><a href='https://evil.com'>Click",
        "token456",
    )
    .expect("email should build");
    let html = email.html_body.unwrap();
    assert!(!html.contains("</p><img"));
    assert!(!html.contains("<img src=x"));
    assert!(!html.contains("</a><a href='https://evil.com'>Click"));
    assert!(!html.contains("href='https://evil.com'"));
    assert!(html.contains("&lt;/p&gt;"));
    assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
    assert!(html.contains("Hacker&lt;/a&gt;&lt;a href=&#39;https://evil.com&#39;&gt;Click"));
}
