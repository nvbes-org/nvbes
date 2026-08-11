use std::{
    ffi::OsString,
    sync::{Mutex, MutexGuard},
};

static ENVIRONMENT_LOCK: Mutex<()> = Mutex::new(());

pub(crate) struct ScopedEnvironment {
    saved: Vec<(&'static str, Option<OsString>)>,
    _lock: MutexGuard<'static, ()>,
}

impl ScopedEnvironment {
    pub(crate) fn replace(values: &[(&'static str, Option<&str>)]) -> Self {
        let lock = ENVIRONMENT_LOCK.lock().expect("environment lock");
        let saved = values
            .iter()
            .map(|(name, _)| (*name, std::env::var_os(name)))
            .collect();
        for (name, value) in values {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
        Self { saved, _lock: lock }
    }
}

impl Drop for ScopedEnvironment {
    fn drop(&mut self) {
        for (name, value) in &self.saved {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
    }
}
