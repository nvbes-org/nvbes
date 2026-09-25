use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use data_encoding::BASE32_NOPAD;
use hmac::{Hmac, KeyInit, Mac};
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

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::{
        generate_totp_code, generate_totp_secret, normalize_code, provisioning_uri,
        random_recovery_code, verify_totp_code,
    };

    #[test]
    fn provisioning_uri_uses_standard_otpauth_totp_shape() {
        let uri = provisioning_uri("Nvbes", "rayane@example.com", "JBSWY3DPEHPK3PXP");

        assert!(uri.starts_with("otpauth://totp/Nvbes:rayane%40example.com?"));
        assert!(uri.contains("secret=JBSWY3DPEHPK3PXP"));
        assert!(uri.contains("issuer=Nvbes"));
        assert!(uri.contains("algorithm=SHA1"));
        assert!(uri.contains("digits=6"));
        assert!(uri.contains("period=30"));
    }

    #[test]
    fn totp_round_trip_accepts_window_and_rejects_bad_codes() {
        let secret = generate_totp_secret();
        assert!(secret.len() >= 20);
        let now = Utc.with_ymd_and_hms(2026, 9, 23, 12, 0, 0).unwrap();
        let counter = super::current_counter(now);
        assert_eq!(counter, now.timestamp() as u64 / 30);
        let code = generate_totp_code(&secret, counter);
        assert_eq!(code.len(), 6);
        assert_eq!(verify_totp_code(&secret, &code, now, 1), Some(counter));
        let previous = generate_totp_code(&secret, counter - 1);
        assert_eq!(
            verify_totp_code(&secret, &previous, now, 1),
            Some(counter - 1)
        );
        assert_eq!(
            verify_totp_code(&secret, &format!(" {code} "), now, 1),
            Some(counter)
        );
        assert!(verify_totp_code(&secret, "123", now, 1).is_none());
        assert!(verify_totp_code(&secret, "000000", now, 1).is_none());
        assert!(generate_totp_code("!!!", 0).is_empty());
        assert_eq!(normalize_code("12-34 56"), "123456");
        let recovery = random_recovery_code();
        assert!(!recovery.is_empty());
        assert_ne!(recovery, "xyzzy");
    }

    #[test]
    fn generate_totp_code_matches_rfc_6238_sha1_6_digit_vector() {
        // ASCII secret "12345678901234567890" as BASE32.
        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        assert_eq!(generate_totp_code(secret, 1), "287082");
        assert_eq!(generate_totp_code(secret, 0), "755224");
    }

    #[test]
    fn current_counter_divides_unix_seconds_by_period() {
        assert_eq!(
            super::current_counter(Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap()),
            0
        );
        assert_eq!(
            super::current_counter(Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 29).unwrap()),
            0
        );
        assert_eq!(
            super::current_counter(Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 30).unwrap()),
            1
        );
        assert_eq!(
            super::current_counter(Utc.with_ymd_and_hms(1970, 1, 1, 0, 1, 0).unwrap()),
            2
        );
    }

    #[test]
    fn verify_totp_skips_negative_candidate_counters() {
        let secret = "JBSWY3DPEHPK3PXP";
        let epoch = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap();
        assert!(verify_totp_code(secret, "000000", epoch, 2).is_none());
    }
}
