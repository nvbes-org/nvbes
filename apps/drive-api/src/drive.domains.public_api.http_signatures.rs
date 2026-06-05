use axum::http::{HeaderMap, Method, Uri};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::Utc;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use crate::http::error::AppError;

const SIGNATURE: &str = "signature";
const SIGNATURE_INPUT: &str = "signature-input";
const CONTENT_DIGEST: &str = "content-digest";
const MAX_CLOCK_SKEW_SECS: i64 = 300;

#[derive(Debug)]
pub struct SignatureIdentity {
    pub key_id: String,
}

#[derive(Debug)]
struct SignatureParams {
    label: String,
    raw_value: String,
    components: Vec<String>,
    key_id: String,
    created: i64,
    expires: Option<i64>,
}

pub fn signature_identity(headers: &HeaderMap) -> Result<Option<SignatureIdentity>, AppError> {
    let Some(input) = header(headers, SIGNATURE_INPUT)? else {
        return Ok(None);
    };
    let params = parse_signature_input(input)?;
    Ok(Some(SignatureIdentity {
        key_id: params.key_id,
    }))
}

pub fn verify(
    headers: &HeaderMap,
    method: &Method,
    uri: &Uri,
    public_key: &str,
) -> Result<(), AppError> {
    let input = required_header(headers, SIGNATURE_INPUT)?;
    let signature = required_header(headers, SIGNATURE)?;
    let params = parse_signature_input(input)?;
    validate_time_window(&params)?;
    validate_required_components(&params, method)?;

    let signature_value = parse_signature_header(signature, &params.label)?;
    let base = signature_base(headers, method, uri, &params)?;
    let public_key = decode_public_key(public_key)?;
    let signature = decode_signature(&signature_value)?;

    public_key
        .verify(base.as_bytes(), &signature)
        .map_err(|_| AppError::unauthorized("http_signature_invalid", "HTTP signature is invalid."))
}

fn validate_time_window(params: &SignatureParams) -> Result<(), AppError> {
    let now = Utc::now().timestamp();
    if params.created < now - MAX_CLOCK_SKEW_SECS {
        return Err(AppError::unauthorized(
            "http_signature_expired",
            "HTTP signature is too old.",
        ));
    }
    if params.created > now + MAX_CLOCK_SKEW_SECS {
        return Err(AppError::unauthorized(
            "http_signature_future",
            "HTTP signature was created in the future.",
        ));
    }
    if params.expires.is_some_and(|expires| expires < now) {
        return Err(AppError::unauthorized(
            "http_signature_expired",
            "HTTP signature has expired.",
        ));
    }
    Ok(())
}

fn validate_required_components(params: &SignatureParams, method: &Method) -> Result<(), AppError> {
    require_component(params, "@method")?;
    require_component(params, "@target-uri")?;
    if matches!(*method, Method::POST | Method::PUT | Method::PATCH) {
        require_component(params, "content-digest")?;
    }
    Ok(())
}

fn require_component(params: &SignatureParams, component: &str) -> Result<(), AppError> {
    if params.components.iter().any(|item| item == component) {
        return Ok(());
    }
    Err(AppError::unauthorized(
        "http_signature_component_missing",
        format!("HTTP signature must cover {component}."),
    ))
}

fn signature_base(
    headers: &HeaderMap,
    method: &Method,
    uri: &Uri,
    params: &SignatureParams,
) -> Result<String, AppError> {
    let mut lines = Vec::with_capacity(params.components.len() + 1);
    for component in &params.components {
        let value = match component.as_str() {
            "@method" => method.as_str().to_ascii_lowercase(),
            "@target-uri" => absolute_target_uri(headers, uri)?,
            "content-digest" => required_header(headers, CONTENT_DIGEST)?.to_owned(),
            header_name => required_header(headers, header_name)?.to_owned(),
        };
        lines.push(format!("\"{component}\": {value}"));
    }
    lines.push(format!("\"@signature-params\": {}", params.raw_value));
    Ok(lines.join("\n"))
}

fn absolute_target_uri(headers: &HeaderMap, uri: &Uri) -> Result<String, AppError> {
    if let Some(scheme) = uri.scheme_str() {
        return Ok(format!("{scheme}:{uri}"));
    }
    let host = required_header(headers, "host")?;
    let scheme = header(headers, "x-forwarded-proto")?.unwrap_or("https");
    Ok(format!("{scheme}://{host}{uri}"))
}

fn parse_signature_input(value: &str) -> Result<SignatureParams, AppError> {
    let (label, rest) = value.split_once('=').ok_or_else(invalid_input)?;
    let label = label.trim().to_owned();
    let rest = rest.trim();
    let closing = rest.find(')').ok_or_else(invalid_input)?;
    let components_raw = rest.strip_prefix('(').ok_or_else(invalid_input)?;
    let components = components_raw[..closing - 1]
        .split_whitespace()
        .map(|item| item.trim_matches('"').to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();

    let mut key_id = None;
    let mut created = None;
    let mut expires = None;
    for param in rest[closing + 1..]
        .split(';')
        .filter(|item| !item.is_empty())
    {
        let Some((name, value)) = param.split_once('=') else {
            continue;
        };
        match name.trim() {
            "keyid" => key_id = Some(value.trim().trim_matches('"').to_owned()),
            "created" => created = value.trim().parse::<i64>().ok(),
            "expires" => expires = value.trim().parse::<i64>().ok(),
            _ => {}
        }
    }

    Ok(SignatureParams {
        label,
        raw_value: rest.to_owned(),
        components,
        key_id: key_id
            .filter(|value| !value.is_empty())
            .ok_or_else(invalid_input)?,
        created: created.ok_or_else(invalid_input)?,
        expires,
    })
}

fn parse_signature_header(value: &str, label: &str) -> Result<String, AppError> {
    for part in value.split(',') {
        let Some((part_label, part_value)) = part.trim().split_once('=') else {
            continue;
        };
        if part_label.trim() == label {
            return Ok(part_value
                .trim()
                .trim_start_matches(':')
                .trim_end_matches(':')
                .to_owned());
        }
    }
    Err(invalid_signature())
}

fn decode_public_key(value: &str) -> Result<VerifyingKey, AppError> {
    let bytes = STANDARD.decode(value).map_err(|_| invalid_key())?;
    let key: [u8; 32] = bytes.try_into().map_err(|_| invalid_key())?;
    VerifyingKey::from_bytes(&key).map_err(|_| invalid_key())
}

fn decode_signature(value: &str) -> Result<Signature, AppError> {
    let bytes = STANDARD.decode(value).map_err(|_| invalid_signature())?;
    let signature: [u8; 64] = bytes.try_into().map_err(|_| invalid_signature())?;
    Ok(Signature::from_bytes(&signature))
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Result<Option<&'a str>, AppError> {
    headers
        .get(name)
        .map(|value| {
            value.to_str().map_err(|_| {
                AppError::unauthorized(
                    "http_signature_header_invalid",
                    format!("HTTP signature header {name} is invalid."),
                )
            })
        })
        .transpose()
}

fn required_header<'a>(headers: &'a HeaderMap, name: &str) -> Result<&'a str, AppError> {
    header(headers, name)?.ok_or_else(|| {
        AppError::unauthorized(
            "http_signature_header_missing",
            format!("HTTP signature requires header {name}."),
        )
    })
}

fn invalid_input() -> AppError {
    AppError::unauthorized(
        "http_signature_input_invalid",
        "Signature-Input header is invalid.",
    )
}

fn invalid_signature() -> AppError {
    AppError::unauthorized("http_signature_invalid", "HTTP signature is invalid.")
}

fn invalid_key() -> AppError {
    AppError::unauthorized(
        "http_signature_key_invalid",
        "HTTP signature public key is invalid.",
    )
}

#[cfg(test)]
#[path = "drive.domains.public_api.http_signatures.tests.rs"]
mod tests;
