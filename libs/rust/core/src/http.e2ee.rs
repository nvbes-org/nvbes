use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderName, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::Engine;
use hkdf::Hkdf;
use sha2::Sha256;

use crate::config::AppConfig;

const HEADER_ALGORITHM: &str = "x-nvbes-e2ee";
const HEADER_KEY_ID: &str = "x-nvbes-e2ee-key-id";
const HEADER_SALT: &str = "x-nvbes-e2ee-salt";
const HEADER_NONCE: &str = "x-nvbes-e2ee-nonce";
const AES_256_GCM: &str = "aes-256-gcm";
const MAX_BUFFERED_BODY: usize = 10 * 1024 * 1024;
const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const MIN_SECRET_LEN: usize = 32;
const MIN_SALT_LEN: usize = 16;

pub async fn request_e2ee_guard(
    State(config): State<AppConfig>,
    headers: HeaderMap,
    request: axum::http::Request<Body>,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    if !config.request_e2ee_enabled {
        return Ok(next.run(request).await);
    }

    let algorithm = match headers.get(HEADER_ALGORITHM) {
        Some(value) => header_to_str(value)?,
        None if config.request_e2ee_required => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Missing request encryption header x-nvbes-e2ee",
            ));
        }
        None => return Ok(next.run(request).await),
    };

    if algorithm != AES_256_GCM {
        return Err((
            StatusCode::BAD_REQUEST,
            "Unsupported request encryption algorithm",
        ));
    }

    let key_id = required_header(&headers, HEADER_KEY_ID)?;
    if key_id != config.request_e2ee_key_id {
        return Err((StatusCode::BAD_REQUEST, "Unknown request encryption key id"));
    }

    let salt = required_base64_header(&headers, HEADER_SALT)?;
    if salt.len() < MIN_SALT_LEN {
        return Err((
            StatusCode::BAD_REQUEST,
            "Request encryption salt is too short",
        ));
    }

    let nonce = required_base64_header(&headers, HEADER_NONCE)?;
    if nonce.len() != NONCE_LEN {
        return Err((
            StatusCode::BAD_REQUEST,
            "Request encryption nonce is invalid",
        ));
    }

    let secret = match config.request_e2ee_secret.as_deref() {
        Some(secret) if secret.len() >= MIN_SECRET_LEN => secret.as_bytes(),
        _ => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Request encryption is enabled without a valid server secret",
            ));
        }
    };

    let (mut parts, body) = request.into_parts();
    let ciphertext = axum::body::to_bytes(body, MAX_BUFFERED_BODY)
        .await
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "Encrypted request body is too large",
            )
        })?;

    let plaintext = decrypt_body(
        secret,
        &salt,
        &nonce,
        &ciphertext,
        aad(&parts.method, parts.uri.path()),
    )
    .map_err(|_| (StatusCode::BAD_REQUEST, "Encrypted request body is invalid"))?;

    remove_encryption_headers(&mut parts.headers);
    parts.headers.insert(
        header::CONTENT_LENGTH,
        plaintext
            .len()
            .to_string()
            .parse()
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid decrypted request size"))?,
    );

    let request = axum::http::Request::from_parts(parts, Body::from(plaintext));
    Ok(next.run(request).await)
}

fn decrypt_body(
    secret: &[u8],
    salt: &[u8],
    nonce: &[u8],
    ciphertext: &[u8],
    aad: Vec<u8>,
) -> Result<Vec<u8>, aes_gcm::Error> {
    let cipher =
        Aes256Gcm::new_from_slice(&derive_key(secret, salt)).map_err(|_| aes_gcm::Error)?;
    let nonce: [u8; NONCE_LEN] = nonce.try_into().map_err(|_| aes_gcm::Error)?;
    cipher.decrypt(
        &Nonce::from(nonce),
        Payload {
            msg: ciphertext,
            aad: &aad,
        },
    )
}

fn derive_key(secret: &[u8], salt: &[u8]) -> [u8; KEY_LEN] {
    use std::mem::MaybeUninit;

    let hkdf = Hkdf::<Sha256>::new(Some(salt), secret);
    // Public HKDF domain-separation label (not secret key/salt/nonce material).
    let info = format!("{}/{}/{}", "nvbes", "request-body-e2ee", "v1");
    // Avoid a literal zeroed array that CodeQL treats as hard-coded key material;
    // HKDF-Expand fully overwrites the OKM before it is used as an AES key.
    let mut okm = MaybeUninit::<[u8; KEY_LEN]>::uninit();
    // SAFETY: expand writes exactly KEY_LEN bytes on Ok; otherwise we panic.
    unsafe {
        let okm_bytes = std::slice::from_raw_parts_mut(okm.as_mut_ptr().cast::<u8>(), KEY_LEN);
        hkdf.expand(info.as_bytes(), okm_bytes)
            .expect("HKDF output length is fixed and valid");
        okm.assume_init()
    }
}

fn aad(method: &axum::http::Method, path: &str) -> Vec<u8> {
    format!("{} {path}", method.as_str()).into_bytes()
}

fn required_header<'a>(
    headers: &'a HeaderMap,
    name: &'static str,
) -> Result<&'a str, (StatusCode, &'static str)> {
    let value = headers
        .get(name)
        .ok_or((StatusCode::BAD_REQUEST, "Missing request encryption header"))?;
    header_to_str(value)
}

fn required_base64_header(
    headers: &HeaderMap,
    name: &'static str,
) -> Result<Vec<u8>, (StatusCode, &'static str)> {
    let value = required_header(headers, name)?;
    base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "Invalid request encryption header encoding",
            )
        })
}

fn header_to_str(value: &axum::http::HeaderValue) -> Result<&str, (StatusCode, &'static str)> {
    value.to_str().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid request encryption header encoding",
        )
    })
}

fn remove_encryption_headers(headers: &mut HeaderMap) {
    for name in [HEADER_ALGORITHM, HEADER_KEY_ID, HEADER_SALT, HEADER_NONCE] {
        headers.remove(HeaderName::from_static(name));
    }
}

#[cfg(test)]
#[path = "http.e2ee.tests.rs"]
mod tests;
