use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::cockpit_model::BackupRestoreSummary;

const RPO_TARGET_HOURS: u32 = 24;
const RTO_TARGET_HOURS: u32 = 8;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RestoreVerificationRecord {
    pub verification_id: String,
    pub backup_timestamp: DateTime<Utc>,
    pub verified_at: DateTime<Utc>,
    pub duration_minutes: u32,
    pub operator: String,
    pub target_environment: String,
    pub schema_checks_passed: bool,
    pub integrity_checks_passed: bool,
}

pub struct BackupRestoreMonitor;

impl BackupRestoreMonitor {
    pub fn evaluate(
        latest_backup_at: Option<DateTime<Utc>>,
        last_restore_verified_at: Option<DateTime<Utc>>,
        last_restore_duration_minutes: Option<u32>,
        integrity_passed: bool,
    ) -> BackupRestoreSummary {
        let now = Utc::now();
        let meets_rpo = match latest_backup_at {
            Some(ts) => (now - ts).num_hours() <= RPO_TARGET_HOURS as i64,
            None => false,
        };

        let meets_rto = match last_restore_duration_minutes {
            Some(mins) => mins <= (RTO_TARGET_HOURS * 60),
            None => false,
        };

        let restore_verification_passed = integrity_passed && meets_rto;

        BackupRestoreSummary {
            latest_backup_at,
            rpo_target_hours: RPO_TARGET_HOURS,
            rto_target_hours: RTO_TARGET_HOURS,
            last_restore_verified_at,
            restore_verification_passed,
            meets_rpo,
            meets_rto,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn validates_backup_within_rpo_and_rto() {
        let now = Utc::now();
        let backup_ts = now - Duration::hours(6);
        let summary = BackupRestoreMonitor::evaluate(
            Some(backup_ts),
            Some(now - Duration::days(2)),
            Some(120), // 2 hours <= 8 hours
            true,
        );

        assert!(summary.meets_rpo);
        assert!(summary.meets_rto);
        assert!(summary.restore_verification_passed);
    }

    #[test]
    fn detects_stale_backup_violating_rpo() {
        let now = Utc::now();
        let stale_backup = now - Duration::hours(36);
        let summary = BackupRestoreMonitor::evaluate(Some(stale_backup), Some(now), Some(60), true);

        assert!(!summary.meets_rpo);
    }
}
