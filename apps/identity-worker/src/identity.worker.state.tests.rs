use std::sync::Mutex;

use super::inline_housekeeping_enabled;

static ENV_LOCK: Mutex<()> = Mutex::new(());
const ENV: &str = "NVBES_IDENTITY_WORKER_INLINE_HOUSEKEEPING_ENABLED";

#[test]
fn inline_housekeeping_defaults_to_enabled_and_accepts_booleans() {
    let _guard = ENV_LOCK.lock().expect("environment lock");
    remove();
    assert!(inline_housekeeping_enabled().expect("default"));
    set("false");
    assert!(!inline_housekeeping_enabled().expect("false"));
    set("true");
    assert!(inline_housekeeping_enabled().expect("true"));
    remove();
}

#[test]
fn inline_housekeeping_rejects_non_boolean_values() {
    let _guard = ENV_LOCK.lock().expect("environment lock");
    set("yes");
    let error = inline_housekeeping_enabled().expect_err("invalid boolean");
    assert!(error.to_string().contains("must be true or false"));
    remove();
}

fn set(value: &str) {
    unsafe { std::env::set_var(ENV, value) };
}

fn remove() {
    unsafe { std::env::remove_var(ENV) };
}
