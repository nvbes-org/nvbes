use super::types::QuotaAlertView;

pub const BYTES_PER_GB: i64 = 1024 * 1024 * 1024;
pub const STORAGE_WARNING_THRESHOLD_PERCENT: i64 = 80;
pub const STORAGE_CRITICAL_THRESHOLD_PERCENT: i64 = 100;

pub fn upload_blocked(used_storage_bytes: i64, included_storage_bytes: i64) -> bool {
    included_storage_bytes <= 0 || used_storage_bytes >= included_storage_bytes
}

pub fn billing_status_blocks_upload(status: &str) -> bool {
    status == "suspended"
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

#[cfg(test)]
mod tests {
    use super::{
        BYTES_PER_GB, billing_status_blocks_upload, crosses_threshold, storage_alerts,
        storage_usage_percent, upload_blocked,
    };

    #[test]
    fn upload_blocked_only_at_or_over_limit() {
        assert!(!upload_blocked(9 * BYTES_PER_GB, 10 * BYTES_PER_GB));
        assert!(upload_blocked(10 * BYTES_PER_GB, 10 * BYTES_PER_GB));
        assert!(upload_blocked(1, 0));
    }

    #[test]
    fn billing_status_blocks_only_suspended_uploads() {
        assert!(!billing_status_blocks_upload("active"));
        assert!(!billing_status_blocks_upload("past_due"));
        assert!(!billing_status_blocks_upload("canceled"));
        assert!(billing_status_blocks_upload("suspended"));
    }

    #[test]
    fn storage_alerts_emit_warning_and_critical_levels() {
        let warning = storage_alerts(8 * BYTES_PER_GB, 10 * BYTES_PER_GB);
        assert_eq!(warning.len(), 1);
        assert_eq!(warning[0].code, "storage_quota_warning");

        let critical = storage_alerts(10 * BYTES_PER_GB, 10 * BYTES_PER_GB);
        assert_eq!(critical.len(), 2);
        assert_eq!(critical[1].code, "storage_quota_critical");
    }

    #[test]
    fn storage_usage_percent_rounds_to_two_decimals() {
        assert_eq!(storage_usage_percent(1, 3), 33.33);
        assert_eq!(storage_usage_percent(1, 0), 100.0);
    }

    #[test]
    fn crosses_threshold_only_when_entering_threshold() {
        assert!(crosses_threshold(
            7 * BYTES_PER_GB,
            8 * BYTES_PER_GB,
            10 * BYTES_PER_GB,
            80
        ));
        assert!(!crosses_threshold(
            8 * BYTES_PER_GB,
            9 * BYTES_PER_GB,
            10 * BYTES_PER_GB,
            80
        ));
        assert!(!crosses_threshold(1, 2, 0, 80));
    }
}
