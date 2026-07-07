use super::*;

const SECRET: &str = "test_jwt_secret_for_bot_guard";
const ACTION: &str = "login_identifier";

fn valid_sig(form_ts: i64) -> String {
    let key = derive_key(SECRET);
    let msg = format!("{form_ts}:{ACTION}");
    compute_hmac(&key, &msg)
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[test]
fn honeypot_empty_passes() {
    let proof = BotGuardProof {
        website: Some(String::new()),
        form_ts: None,
        js_sig: None,
    };
    assert!(check_decoy_field(&proof).is_ok());
}

#[test]
fn honeypot_filled_fails() {
    let proof = BotGuardProof {
        website: Some("http://spam.example.com".into()),
        form_ts: None,
        js_sig: None,
    };
    let err = check_decoy_field(&proof).unwrap_err();
    assert_eq!(err.code, "bot_honeypot_triggered");
}

#[test]
fn honeypot_email_pattern_fails() {
    let proof = BotGuardProof {
        website: Some("spammer@evil.com".into()),
        form_ts: None,
        js_sig: None,
    };
    let err = check_decoy_field(&proof).unwrap_err();
    assert_eq!(err.code, "bot_honeypot_triggered");
}

#[test]
fn honeypot_phone_pattern_fails() {
    let proof = BotGuardProof {
        website: Some("+33612345678".into()),
        form_ts: None,
        js_sig: None,
    };
    let err = check_decoy_field(&proof).unwrap_err();
    assert_eq!(err.code, "bot_honeypot_triggered");
}

#[test]
fn time_lock_normal_passes() {
    let form_ts = now_ms() - 3_000;
    assert!(check_time_lock(form_ts, now_ms()).is_ok());
}

#[test]
fn time_lock_too_fast_fails() {
    let form_ts = now_ms() - 500;
    let err = check_time_lock(form_ts, now_ms()).unwrap_err();
    assert_eq!(err.code, "bot_time_lock_fast");
}

#[test]
fn time_lock_stale_fails() {
    let form_ts = now_ms() - (MAX_FORM_AGE_MS + 1_000);
    let err = check_time_lock(form_ts, now_ms()).unwrap_err();
    assert_eq!(err.code, "bot_time_lock_expired");
}

#[test]
fn js_signature_valid_passes() {
    let form_ts = now_ms() - 2_000;
    let sig = valid_sig(form_ts);
    assert!(check_js_signature(&sig, form_ts, ACTION, SECRET).is_ok());
}

#[test]
fn js_signature_wrong_fails() {
    let form_ts = now_ms() - 2_000;
    let err = check_js_signature("deadbeef", form_ts, ACTION, SECRET).unwrap_err();
    assert_eq!(err.code, "bot_js_signature_invalid");
}

#[test]
fn js_signature_wrong_action_fails() {
    let form_ts = now_ms() - 2_000;
    let sig = valid_sig(form_ts);
    let err = check_js_signature(&sig, form_ts, "register", SECRET).unwrap_err();
    assert_eq!(err.code, "bot_js_signature_invalid");
}

#[test]
fn full_verify_passes() {
    let form_ts = now_ms() - 2_000;
    let sig = valid_sig(form_ts);
    let proof = BotGuardProof {
        website: Some(String::new()),
        form_ts: Some(form_ts),
        js_sig: Some(sig),
    };
    assert!(verify(&proof, ACTION, SECRET).is_ok());
}
