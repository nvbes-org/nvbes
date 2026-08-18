use std::sync::Mutex;

use super::{
    DEFAULT_IDENTITY_WORKER_METRICS_BIND_ADDR, IDENTITY_WORKER_METRICS_BIND_ADDR_ENV,
    WORKER_METRICS_BIND_ADDR_ENV, identity_worker_metrics_bind_addr,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn metrics_address_uses_identity_then_shared_then_default_precedence() {
    let _guard = ENV_LOCK.lock().expect("environment lock");
    clear();
    assert_eq!(
        identity_worker_metrics_bind_addr(),
        DEFAULT_IDENTITY_WORKER_METRICS_BIND_ADDR
    );
    set(WORKER_METRICS_BIND_ADDR_ENV, "127.0.0.1:5000");
    assert_eq!(identity_worker_metrics_bind_addr(), "127.0.0.1:5000");
    set(IDENTITY_WORKER_METRICS_BIND_ADDR_ENV, "127.0.0.1:5001");
    assert_eq!(identity_worker_metrics_bind_addr(), "127.0.0.1:5001");
    clear();
}

fn set(name: &str, value: &str) {
    unsafe { std::env::set_var(name, value) };
}

fn clear() {
    unsafe {
        std::env::remove_var(IDENTITY_WORKER_METRICS_BIND_ADDR_ENV);
        std::env::remove_var(WORKER_METRICS_BIND_ADDR_ENV);
    }
}
