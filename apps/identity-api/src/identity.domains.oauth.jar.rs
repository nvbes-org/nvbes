use crate::http::error::AppError;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode_header};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestObjectClaims {
    pub iss: String,
    pub aud: String,
    pub exp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbf: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iat: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,
    pub response_type: Option<String>,
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub audience: Option<String>,
    pub resource: Option<Vec<String>>,
    pub authorization_details: Option<serde_json::Value>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub consent_action: Option<String>,
}

pub fn validate_request_object(
    request_jwt: &str,
    client_secret: &str,
    client_id: &str,
    issuer_url: &str,
) -> Result<RequestObjectClaims, AppError> {
    let header = decode_header(request_jwt).map_err(|e| {
        AppError::bad_request(
            "invalid_request_object",
            &format!("Invalid request object header: {}", e),
        )
    })?;

    let algorithm = match header.alg {
        Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => header.alg,
        _ => {
            return Err(AppError::bad_request(
                "invalid_request_object",
                "Unsupported request object signing algorithm. Only HS256, HS384, HS512 are supported.",
            ));
        }
    };

    let decoding_key = DecodingKey::from_secret(client_secret.as_bytes());
    let mut validation = Validation::new(algorithm);
    validation.set_issuer(&[client_id]);
    validation.set_audience(&[issuer_url]);
    validation.validate_exp = true;
    validation.validate_nbf = true;
    validation.leeway = 0;
    validation.set_required_spec_claims(&["iss", "aud", "exp"]);

    let token_data =
        jsonwebtoken::decode::<RequestObjectClaims>(request_jwt, &decoding_key, &validation)
            .map_err(|e| {
                let message = match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                        "Request object has expired"
                    }
                    jsonwebtoken::errors::ErrorKind::InvalidIssuer => {
                        "Invalid request object issuer"
                    }
                    jsonwebtoken::errors::ErrorKind::InvalidAudience => {
                        "Invalid request object audience"
                    }
                    jsonwebtoken::errors::ErrorKind::ImmatureSignature => {
                        "Request object is not yet valid (nbf)"
                    }
                    jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                        "Invalid request object signature"
                    }
                    _ => "",
                };
                if message.is_empty() {
                    AppError::bad_request(
                        "invalid_request_object",
                        &format!("Invalid request object: {}", e),
                    )
                } else {
                    AppError::bad_request("invalid_request_object", message)
                }
            })?;

    let claims = token_data.claims;

    if let Some(req_resp_type) = claims.response_type.as_deref() {
        if req_resp_type != "code" {
            return Err(AppError::bad_request(
                "invalid_request_object",
                "Only response_type=code is supported",
            ));
        }
    }

    if let Some(ref req_client_id) = claims.client_id {
        if req_client_id != client_id {
            return Err(AppError::bad_request(
                "invalid_request_object",
                "client_id in request object does not match the request",
            ));
        }
    }

    Ok(claims)
}

pub fn is_par_urn(request_uri: &str) -> bool {
    request_uri.starts_with("urn:ietf:params:oauth:request_uri:gxpar_")
}

pub async fn fetch_request_object_from_uri(uri: &str) -> Result<String, AppError> {
    let parsed = url::Url::parse(uri).map_err(|_| {
        AppError::bad_request("invalid_request_uri", "The request_uri is not a valid URL.")
    })?;

    if parsed.scheme() != "https" {
        return Err(AppError::bad_request(
            "invalid_request_uri",
            "request_uri must use HTTPS.",
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| AppError::internal("http_client_error", &e.to_string()))?;

    let request = client.get(uri);
    let response = nvbes_core::trace_context::with_fresh_trace_headers(request)
        .send()
        .await
        .map_err(|e| {
            AppError::bad_request(
                "invalid_request_uri",
                &format!("Failed to fetch request_uri: {}", e),
            )
        })?;

    if !response.status().is_success() {
        return Err(AppError::bad_request(
            "invalid_request_uri",
            &format!("Failed to fetch request_uri: HTTP {}", response.status()),
        ));
    }

    let body = response.text().await.map_err(|e| {
        AppError::bad_request(
            "invalid_request_uri",
            &format!("Failed to read request_uri response: {}", e),
        )
    })?;

    let trimmed = body.trim().to_string();
    if trimmed.is_empty() {
        return Err(AppError::bad_request(
            "invalid_request_uri",
            "request_uri response is empty",
        ));
    }

    Ok(trimmed)
}

#[cfg(test)]
#[path = "identity.domains.oauth.jar.tests.rs"]
mod tests;
