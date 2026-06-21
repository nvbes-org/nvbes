use nvbes_core::config::AppConfig;

#[derive(Debug, Default)]
pub struct ErrorReportingGuard;

pub fn init_error_reporting(_config: &AppConfig) -> ErrorReportingGuard {
    ErrorReportingGuard
}

pub fn install_safe_panic_hook() {}
