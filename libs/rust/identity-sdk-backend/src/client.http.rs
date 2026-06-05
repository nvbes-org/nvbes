use reqwest::{RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;

use super::IdentityClient;
use crate::SdkError;

impl IdentityClient {
    pub(crate) fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url.trim_end_matches('/'), path)
    }

    pub(crate) async fn send_json<T>(&self, request: RequestBuilder) -> Result<T, SdkError>
    where
        T: DeserializeOwned,
    {
        let response = request.send().await?;
        if !response.status().is_success() {
            return Err(auth_error(response).await);
        }

        Ok(response.json().await?)
    }

    pub(crate) async fn send_empty(&self, request: RequestBuilder) -> Result<(), SdkError> {
        let response = request.send().await?;
        if !response.status().is_success() {
            return Err(auth_error(response).await);
        }

        Ok(())
    }

    pub(crate) async fn send_oauth_json<T>(&self, request: RequestBuilder) -> Result<T, SdkError>
    where
        T: DeserializeOwned,
    {
        let response = request.send().await?;
        if !response.status().is_success() {
            return Err(SdkError::TokenExchange(error_body(response).await));
        }

        Ok(response.json().await?)
    }
}

pub(crate) async fn auth_error(response: Response) -> SdkError {
    let status = response.status().as_u16();
    SdkError::auth(error_body(response).await, Some(status))
}

pub(crate) async fn error_body(response: Response) -> String {
    response.text().await.unwrap_or_default()
}

pub(crate) fn is_accepted(status: StatusCode) -> bool {
    status == StatusCode::ACCEPTED
}
