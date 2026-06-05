use std::sync::Arc;

use nvbes_core::config::AppConfig;

#[path = "sentry.scrub.rs"]
mod scrub;

pub fn init_sentry(config: &AppConfig) -> sentry::ClientInitGuard {
    let dsn = config.sentry_dsn.as_deref().unwrap_or("");

    let traces_sample_rate = if config.environment == "production" {
        0.2
    } else {
        1.0
    };

    sentry::init((
        dsn,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            environment: Some(config.environment.clone().into()),
            traces_sample_rate,
            enable_logs: config.sentry_logs_enabled,
            send_default_pii: false,
            before_send: Some(Arc::new(scrub::scrub_event)),
            before_breadcrumb: Some(Arc::new(scrub::scrub_breadcrumb)),
            before_send_log: Some(Arc::new(scrub::scrub_log)),
            ..Default::default()
        },
    ))
}

pub fn install_safe_panic_hook() {
    let sentry_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let location = panic_info
            .location()
            .map(|l| l.to_string())
            .unwrap_or_default();
        let msg = panic_info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| panic_info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic".to_string());

        if location.contains("sentry-tracing")
            || location.contains("tracing-subscriber")
            || msg.contains("SentryLayer")
            || msg.contains("tried to clone a span")
            || msg.contains("already closed")
        {
            let msg_truncated: String = msg
                .chars()
                .take(200)
                .chain(if msg.len() > 200 { "..." } else { "" }.chars())
                .collect();
            eprintln!(
                "[sentry] known upstream bug suppressed (not a system crash): {}",
                msg_truncated
            );
            return;
        }

        sentry_hook(panic_info);
    }));
}
