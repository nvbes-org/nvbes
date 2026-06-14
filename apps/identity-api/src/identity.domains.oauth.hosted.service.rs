use chrono::Utc;
use uuid::Uuid;

use super::{
    hosted_store::{get_hosted_authorization_state, set_hosted_authorization_state},
    hosted_types::{CachedHostedAuthorizationState, HostedClientDisplay, HostedLoginDecision},
};
use crate::http::error::AppError;

pub struct StartHostedAuthorizationInput {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub request_uri: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

pub fn build_hosted_login_url(identity_web_base_url: &str, state_id: &str) -> String {
    let mut url = url::Url::parse(identity_web_base_url.trim_end_matches('/'))
        .expect("identity web base URL must be absolute");
    url.set_path("/login");
    url.query_pairs_mut().append_pair("state_id", state_id);
    url.to_string()
}

pub fn build_oauth_redirect_url(
    redirect_uri: &str,
    params: &[(&str, &str)],
) -> Result<String, AppError> {
    let mut url = url::Url::parse(redirect_uri).map_err(|_| {
        AppError::bad_request("invalid_redirect_uri", "The redirect_uri is invalid.")
    })?;
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in params {
            query.append_pair(key, value);
        }
    }
    Ok(url.to_string())
}

pub fn build_oauth_error_redirect_url(
    redirect_uri: &str,
    error: &str,
    error_description: &str,
    state: Option<&str>,
) -> Result<String, AppError> {
    let mut params = vec![("error", error), ("error_description", error_description)];
    if let Some(state) = state {
        params.push(("state", state));
    }
    build_oauth_redirect_url(redirect_uri, &params)
}

pub async fn create_hosted_authorization_state(
    redis: &nvbes_redis::RedisPool,
    input: StartHostedAuthorizationInput,
) -> Result<CachedHostedAuthorizationState, AppError> {
    let now = Utc::now();
    let state = CachedHostedAuthorizationState {
        state_id: format!("hosted_{}", Uuid::new_v4().simple()),
        client_id: input.client_id,
        redirect_uri: input.redirect_uri,
        scope: input
            .scope
            .unwrap_or_else(|| "openid profile email".to_string()),
        state: input.state,
        request_uri: input.request_uri,
        code_challenge: input.code_challenge,
        code_challenge_method: input.code_challenge_method,
        tenant_id: None,
        workspace_id: None,
        created_at: now,
        expires_at: now + chrono::Duration::seconds(300),
    };
    set_hosted_authorization_state(redis, &state).await?;
    Ok(state)
}

pub async fn get_hosted_login_decision(
    redis: &nvbes_redis::RedisPool,
    state_id: &str,
) -> Result<HostedLoginDecision, AppError> {
    let Some(state) = get_hosted_authorization_state(redis, state_id).await? else {
        return Ok(HostedLoginDecision::ErrorPage {
            code: "invalid_request".to_string(),
            message: "This login request is no longer valid.".to_string(),
        });
    };

    Ok(HostedLoginDecision::ConsentRequired {
        state_id: state.state_id,
        client: HostedClientDisplay {
            client_id: state.client_id.clone(),
            name: state.client_id,
        },
        scope: state.scope,
    })
}
