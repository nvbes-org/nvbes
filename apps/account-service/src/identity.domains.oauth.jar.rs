use crate::http::error::AppError;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode_header};
use serde::{Deserialize, Serialize};

#[path = "identity.domains.oauth.jar.high_assurance.rs"]
mod high_assurance;

pub use high_assurance::{
    record_request_object_jti, validate_high_assurance_jwks, validate_high_assurance_request_object,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestObjectClaims {
    pub iss: String,
    pub aud: RequestObjectAudience,
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
    pub nonce: Option<String>,
    pub audience: Option<String>,
    pub resource: Option<Vec<String>>,
    pub authorization_details: Option<serde_json::Value>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub consent_action: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RequestObjectAudience {
    One(String),
    Many(Vec<String>),
}

impl RequestObjectAudience {
    pub fn contains(&self, expected: &str) -> bool {
        match self {
            Self::One(audience) => audience == expected,
            Self::Many(audiences) => audiences.iter().any(|audience| audience == expected),
        }
    }
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
            format!("Invalid request object header: {}", e),
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
                        format!("Invalid request object: {}", e),
                    )
                } else {
                    AppError::bad_request("invalid_request_object", message)
                }
            })?;

    let claims = token_data.claims;

    if let Some(req_resp_type) = claims.response_type.as_deref()
        && req_resp_type != "code"
    {
        return Err(AppError::bad_request(
            "invalid_request_object",
            "Only response_type=code is supported",
        ));
    }

    if let Some(ref req_client_id) = claims.client_id
        && req_client_id != client_id
    {
        return Err(AppError::bad_request(
            "invalid_request_object",
            "client_id in request object does not match the request",
        ));
    }

    Ok(claims)
}

pub fn is_par_urn(request_uri: &str) -> bool {
    request_uri.starts_with("urn:ietf:params:oauth:request_uri:gxpar_")
}

#[cfg(test)]
#[path = "identity.domains.oauth.jar.tests.rs"]
mod tests;
