use super::types::QuotaAlertView;

pub const BYTES_PER_GB: i64 = 1024 * 1024 * 1024;
pub const STORAGE_WARNING_THRESHOLD_PERCENT: i64 = 80;
pub const STORAGE_CRITICAL_THRESHOLD_PERCENT: i64 = 100;

pub fn upload_blocked(used_storage_bytes: i64, included_storage_bytes: i64) -> bool {
    included_storage_bytes <= 0 || used_storage_bytes >= included_storage_bytes
}

pub fn storage_usage_percent(used_storage_bytes: i64, included_storage_bytes: i64) -> f64 {
    if included_storage_bytes <= 0 {
        return 100.0;
    }

    ((used_storage_bytes as f64 / included_storage_bytes as f64) * 10_000.0).round() / 100.0
}

pub fn storage_alerts(used_storage_bytes: i64, included_storage_bytes: i64) -> Vec<QuotaAlertView> {
    let percent = storage_usage_percent(used_storage_bytes, included_storage_bytes);
    let mut alerts = Vec::new();

    if percent >= STORAGE_WARNING_THRESHOLD_PERCENT as f64 {
        alerts.push(QuotaAlertView {
            code: "storage_quota_warning",
            level: "warning",
            threshold_percent: STORAGE_WARNING_THRESHOLD_PERCENT,
            message: "Workspace storage usage is at or above 80%.",
        });
    }

    if percent >= STORAGE_CRITICAL_THRESHOLD_PERCENT as f64 {
        alerts.push(QuotaAlertView {
            code: "storage_quota_critical",
            level: "critical",
            threshold_percent: STORAGE_CRITICAL_THRESHOLD_PERCENT,
            message: "Workspace storage usage is at or above 100%; uploads are blocked.",
        });
    }

    alerts
}

pub fn crosses_threshold(
    before_used_storage_bytes: i64,
    after_used_storage_bytes: i64,
    included_storage_bytes: i64,
    threshold_percent: i64,
) -> bool {
    if included_storage_bytes <= 0 {
        return false;
    }

    let threshold_bytes = included_storage_bytes.saturating_mul(threshold_percent) / 100;
    before_used_storage_bytes < threshold_bytes && after_used_storage_bytes >= threshold_bytes
}
