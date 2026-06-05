use std::sync::OnceLock;
use std::time::Duration;

use reqwest::tls::Version;

static PINNED_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub fn pinned_http_client() -> reqwest::Client {
    PINNED_CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .min_tls_version(Version::TLS_1_3)
                .connect_timeout(CONNECT_TIMEOUT)
                .timeout(REQUEST_TIMEOUT)
                .https_only(true)
                .build()
                .expect("pinned TLS client should initialize")
        })
        .clone()
}
