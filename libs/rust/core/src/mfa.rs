use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use data_encoding::BASE32_NOPAD;
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha1::Sha1;
use urlencoding::encode;

type HmacSha1 = Hmac<Sha1>;

pub const TOTP_DIGITS: u32 = 6;
pub const TOTP_PERIOD_SECONDS: u64 = 30;
pub const TOTP_WINDOW: i64 = 1;

pub fn generate_totp_secret() -> String {
    let mut bytes = [0_u8; 20];
    rand::rng().fill_bytes(&mut bytes);
    BASE32_NOPAD.encode(&bytes)
}

pub fn provisioning_uri(issuer: &str, account_name: &str, secret: &str) -> String {
    let issuer = encode(issuer);
    let account_name = encode(account_name);
    format!(
        "otpauth://totp/{issuer}:{account_name}?secret={secret}&issuer={issuer}&algorithm=SHA1&digits={TOTP_DIGITS}&period={TOTP_PERIOD_SECONDS}"
    )
}

pub fn current_counter(now: DateTime<Utc>) -> u64 {
    let seconds = now.timestamp().max(0) as u64;
    seconds / TOTP_PERIOD_SECONDS
}

pub fn verify_totp_code(secret: &str, code: &str, now: DateTime<Utc>, window: i64) -> Option<u64> {
    let normalized = normalize_code(code);
    if normalized.len() as u32 != TOTP_DIGITS {
        return None;
    }

    let counter = current_counter(now);
    for offset in -window..=window {
        let candidate = counter as i64 + offset;
        if candidate < 0 {
            continue;
        }

        if generate_totp_code(secret, candidate as u64) == normalized {
            return Some(candidate as u64);
        }
    }

    None
}

pub fn generate_totp_code(secret: &str, counter: u64) -> String {
    let secret_bytes = match BASE32_NOPAD.decode(secret.as_bytes()) {
        Ok(bytes) => bytes,
        Err(_) => return String::new(),
    };

    let mut mac = match HmacSha1::new_from_slice(&secret_bytes) {
        Ok(mac) => mac,
        Err(_) => return String::new(),
    };

    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = (digest[19] & 0x0f) as usize;
    let binary = ((u32::from(digest[offset]) & 0x7f) << 24)
        | (u32::from(digest[offset + 1]) << 16)
        | (u32::from(digest[offset + 2]) << 8)
        | u32::from(digest[offset + 3]);
    let digits = 10_u32.pow(TOTP_DIGITS);
    format!("{:0width$}", binary % digits, width = TOTP_DIGITS as usize)
}

pub fn normalize_code(code: &str) -> String {
    code.chars().filter(|ch| ch.is_ascii_digit()).collect()
}

pub fn random_recovery_code() -> String {
    let mut bytes = [0_u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
