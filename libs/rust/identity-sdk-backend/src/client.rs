use reqwest::Client;

use crate::types::AuthConfig;

#[path = "client.auth.rs"]
mod auth;
#[path = "client.http.rs"]
mod http;
#[path = "client.mfa.rs"]
mod mfa;
#[path = "client.oauth.rs"]
mod oauth;
#[path = "client.trace.rs"]
mod trace;

pub struct IdentityClient {
    pub(crate) http: Client,
    pub(crate) config: AuthConfig,
}

impl IdentityClient {
    pub fn new(config: AuthConfig) -> Self {
        Self {
            http: Client::new(),
            config,
        }
    }

    pub fn with_http_client(config: AuthConfig, http: Client) -> Self {
        Self { http, config }
    }
}
