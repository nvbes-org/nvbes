use std::time::Duration;

pub fn record_redis_command(command: &str, outcome: &str, duration: Duration) {
    let labels = [
        ("command", command.to_string()),
        ("outcome", outcome.to_string()),
    ];

    metrics::counter!("redis_commands_total", &labels).increment(1);
    metrics::histogram!("redis_command_duration_seconds", &labels).record(duration.as_secs_f64());
}

pub fn record_cache_hit(cache_type: &str) {
    let labels = [("cache_type", cache_type.to_string())];
    metrics::counter!("redis_cache_hit_total", &labels).increment(1);
}

pub fn record_cache_miss(cache_type: &str) {
    let labels = [("cache_type", cache_type.to_string())];
    metrics::counter!("redis_cache_miss_total", &labels).increment(1);
}

pub fn record_rate_limit_event(bucket: &str, action: &str, allowed: bool) {
    let labels = [
        ("bucket", bucket.to_string()),
        ("action", action.to_string()),
    ];

    if allowed {
        metrics::counter!("redis_ratelimit_allowed_total", &labels).increment(1);
    } else {
        metrics::counter!("redis_ratelimit_blocked_total", &labels).increment(1);
    }
}

pub fn record_pool_stats(available: u32, active: u32) {
    metrics::gauge!("redis_pool_available".to_string()).set(available as f64);
    metrics::gauge!("redis_pool_active".to_string()).set(active as f64);
}
