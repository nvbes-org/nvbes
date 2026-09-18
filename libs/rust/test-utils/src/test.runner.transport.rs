use super::registry::KeywordError;
use std::time::Duration;

pub fn client() -> Result<reqwest::Client, KeywordError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .no_proxy()
        .build()
        .map_err(|_| KeywordError::Execution("test HTTP client initialization failed".into()))
}
