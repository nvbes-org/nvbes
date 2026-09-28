use std::net::SocketAddr;
use std::sync::{Mutex, OnceLock};

use super::{bind_listener_with_keepalive, env_parse_u32, env_parse_u64};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_env() -> std::sync::MutexGuard<'static, ()> {
    env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct EnvGuard {
    saved: Vec<(String, Option<String>)>,
}

impl EnvGuard {
    fn set(pairs: &[(&str, &str)]) -> Self {
        let mut saved = Vec::with_capacity(pairs.len());
        for (name, value) in pairs {
            saved.push(((*name).to_string(), std::env::var(name).ok()));
            // SAFETY: serialized by `env_lock` for test-only env mutation.
            unsafe { std::env::set_var(name, value) };
        }
        Self { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (name, previous) in &self.saved {
            match previous {
                Some(value) => unsafe { std::env::set_var(name, value) },
                None => unsafe { std::env::remove_var(name) },
            }
        }
    }
}

#[test]
fn env_parse_helpers_use_defaults_for_missing_and_invalid_values() {
    let _lock = lock_env();
    let name_u64 = "NVBES_CORE_KEEPALIVE_TEST_U64";
    let name_u32 = "NVBES_CORE_KEEPALIVE_TEST_U32";
    unsafe {
        std::env::remove_var(name_u64);
        std::env::remove_var(name_u32);
    }
    assert_eq!(env_parse_u64(name_u64, 60), 60);
    assert_eq!(env_parse_u32(name_u32, 3), 3);

    let _guard = EnvGuard::set(&[(name_u64, "not-a-number"), (name_u32, "nope")]);
    assert_eq!(env_parse_u64(name_u64, 60), 60);
    assert_eq!(env_parse_u32(name_u32, 3), 3);
}

#[test]
fn env_parse_helpers_accept_valid_overrides() {
    let _lock = lock_env();
    let name_u64 = "NVBES_CORE_KEEPALIVE_TEST_U64_OK";
    let name_u32 = "NVBES_CORE_KEEPALIVE_TEST_U32_OK";
    let _guard = EnvGuard::set(&[(name_u64, "42"), (name_u32, "7")]);
    assert_eq!(env_parse_u64(name_u64, 60), 42);
    assert_eq!(env_parse_u32(name_u32, 3), 7);
}

#[tokio::test]
async fn bind_listener_with_keepalive_accepts_ephemeral_ipv4_port() {
    let _lock = lock_env();
    let _guard = EnvGuard::set(&[
        ("NVBES_TCP_KEEPALIVE_IDLE_SECS", "30"),
        ("NVBES_TCP_KEEPALIVE_INTERVAL_SECS", "5"),
        ("NVBES_TCP_KEEPALIVE_PROBES", "2"),
    ]);

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listener = bind_listener_with_keepalive(addr, 128).expect("bind with keepalive");
    let local = listener.local_addr().expect("local addr");
    assert_eq!(local.ip().to_string(), "127.0.0.1");
    assert_ne!(local.port(), 0);
}
