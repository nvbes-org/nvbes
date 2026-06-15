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
    db: &sqlx::PgPool,
    redis: &nvbes_redis::RedisPool,
    state_id: &str,
) -> Result<HostedLoginDecision, AppError> {
    let Some(state) = get_hosted_authorization_state(redis, state_id).await? else {
        return Ok(HostedLoginDecision::ErrorPage {
            code: "invalid_request".to_string(),
            message: "This login request is no longer valid.".to_string(),
        });
    };

    #[derive(sqlx::FromRow)]
    struct ClientBrandingRow {
        name: String,
        product_name: Option<String>,
        logo_url: Option<String>,
        description: Option<String>,
        support_url: Option<String>,
        privacy_url: Option<String>,
        terms_url: Option<String>,
        brand_color: Option<String>,
        custom_css: Option<String>,
        help_text: Option<String>,
    }

    let client_details = sqlx::query_as::<_, ClientBrandingRow>(
        r#"
        SELECT
          c.name,
          cs.product_name,
          cs.logo_url,
          cs.description,
          cs.support_url,
          cs.privacy_url,
          cs.terms_url,
          cs.brand_color,
          cs.custom_css,
          cs.help_text
        FROM oauth_clients c
        LEFT JOIN developer_consent_screens cs ON cs.client_id = c.client_id
        WHERE c.client_id = $1
        LIMIT 1
        "#,
    )
    .bind(&state.client_id)
    .fetch_optional(db)
    .await?;

    let client_display = match client_details {
        Some(row) => {
            let display_name = row.product_name
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(row.name);
            HostedClientDisplay {
                client_id: state.client_id.clone(),
                name: display_name,
                logo_url: row.logo_url,
                description: row.description,
                support_url: row.support_url,
                privacy_url: row.privacy_url,
                terms_url: row.terms_url,
                brand_color: row.brand_color,
                custom_css: row.custom_css,
                help_text: row.help_text,
            }
        }
        None => HostedClientDisplay {
            client_id: state.client_id.clone(),
            name: state.client_id.clone(),
            logo_url: None,
            description: None,
            support_url: None,
            privacy_url: None,
            terms_url: None,
            brand_color: None,
            custom_css: None,
            help_text: None,
        }
    };

    Ok(HostedLoginDecision::ConsentRequired {
        state_id: state.state_id,
        client: client_display,
        scope: state.scope,
    })
}
