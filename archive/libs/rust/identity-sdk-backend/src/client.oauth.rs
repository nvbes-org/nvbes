use reqwest::header;

use super::IdentityClient;
use crate::types::TokenResponse;
use crate::SdkError;

impl IdentityClient {
    /// Échange un code d'autorisation OAuth2 contre des tokens.
    pub async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
        code_verifier: Option<&str>,
    ) -> Result<TokenResponse, SdkError> {
        let mut params = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", self.config.client_id.as_str()),
        ];

        if let Some(secret) = &self.config.client_secret {
            params.push(("client_secret", secret));
        }

        if let Some(verifier) = code_verifier {
            params.push(("code_verifier", verifier));
        }

        self.send_oauth_json(
            self.http
                .post(self.endpoint("/oauth/token"))
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(form_body(&params)),
        )
        .await
    }

    /// Rafraîchit un access token via refresh token opaque.
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse, SdkError> {
        let mut params = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", self.config.client_id.as_str()),
        ];

        if let Some(secret) = &self.config.client_secret {
            params.push(("client_secret", secret));
        }

        self.send_oauth_json(
            self.http
                .post(self.endpoint("/oauth/token"))
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(form_body(&params)),
        )
        .await
    }
}

fn form_body(params: &[(&str, &str)]) -> String {
    params
        .iter()
        .map(|(key, value)| format!("{}={}", key, urlencoding::encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}
