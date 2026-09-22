use super::*;
use chrono::Duration;

#[test]
fn validates_backup_within_rpo_and_rto() {
    let now = Utc::now();
    let backup_ts = now - Duration::hours(6);
    let summary = BackupRestoreMonitor::evaluate(
        Some(backup_ts),
        Some(now - Duration::days(2)),
        Some(120),
        true,
    );

    assert!(summary.meets_rpo);
    assert!(summary.meets_rto);
    assert!(summary.restore_verification_passed);
}

#[test]
fn detects_stale_backup_and_missing_restore_evidence() {
    let now = Utc::now();
    let stale_backup = now - Duration::hours(36);
    let summary = BackupRestoreMonitor::evaluate(Some(stale_backup), Some(now), Some(60), true);
    assert!(!summary.meets_rpo);

    let missing = BackupRestoreMonitor::evaluate(None, None, None, false);
    assert!(!missing.meets_rpo);
    assert!(!missing.meets_rto);
    assert!(!missing.restore_verification_passed);
}

#[test]
fn long_restore_duration_fails_rto() {
    let now = Utc::now();
    let summary = BackupRestoreMonitor::evaluate(
        Some(now - Duration::hours(1)),
        Some(now),
        Some(RTO_TARGET_HOURS * 60 + 1),
        true,
    );
    assert!(!summary.meets_rto);
    assert!(!summary.restore_verification_passed);
}
