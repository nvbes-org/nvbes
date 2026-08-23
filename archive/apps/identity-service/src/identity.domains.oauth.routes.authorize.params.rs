use crate::http::error::AppError;

#[cfg(test)]
#[path = "identity.domains.oauth.routes.authorize.params.tests.rs"]
mod tests;

#[derive(Debug, Default)]
pub(super) struct ResolvedParams {
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub nonce: Option<String>,
    pub audience: Option<String>,
    pub resource: Option<Vec<String>>,
    pub authorization_details: crate::domains::oauth::rar::AuthorizationDetails,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub dpop_jkt: Option<String>,
}

pub(super) fn build_params_from_map(
    params: &serde_json::Map<String, serde_json::Value>,
) -> Result<ResolvedParams, AppError> {
    let response_type = params.get("response_type").and_then(|value| value.as_str());
    if response_type != Some("code") {
        return Err(AppError::bad_request(
            "invalid_response_type",
            "Only 'code' is supported",
        ));
    }

    let redirect_uri = params
        .get("redirect_uri")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            AppError::bad_request(
                "invalid_request",
                "The pushed authorization request is missing redirect_uri.",
            )
        })?
        .to_string();

    Ok(ResolvedParams {
        redirect_uri,
        scope: params
            .get("scope")
            .and_then(|v| v.as_str())
            .map(String::from),
        state: params
            .get("state")
            .and_then(|v| v.as_str())
            .map(String::from),
        nonce: params
            .get("nonce")
            .and_then(|v| v.as_str())
            .map(String::from),
        audience: params
            .get("audience")
            .and_then(|v| v.as_str())
            .map(String::from),
        resource: params.get("resource").and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
        }),
        authorization_details: crate::domains::oauth::rar::parse_authorization_details_value(
            params.get("authorization_details"),
        )?,
        code_challenge: params
            .get("code_challenge")
            .and_then(|v| v.as_str())
            .map(String::from),
        code_challenge_method: params
            .get("code_challenge_method")
            .and_then(|v| v.as_str())
            .map(String::from),
        dpop_jkt: params
            .get("dpop_jkt")
            .and_then(|value| value.as_str())
            .map(String::from),
    })
}
