use super::*;

#[test]
fn validates_replay_eligibility() {
    let monitor = JobsMonitor::new(3);
    let valid_failed = FailedEventRecord {
        event_id: Uuid::new_v4(),
        event_type: "email.message.dispatched".to_string(),
        correlation_id: Uuid::new_v4(),
        attempt_count: 2,
        last_error: "Temporary network timeout".to_string(),
        state: JobState::Failed,
        failed_at: Utc::now(),
    };
    assert!(monitor.can_replay(&valid_failed).is_ok());

    let exhausted = FailedEventRecord {
        attempt_count: 3,
        ..valid_failed.clone()
    };
    assert_eq!(
        monitor.can_replay(&exhausted),
        Err(JobsMonitorError::MaxAttemptsExceeded(3))
    );

    let completed = FailedEventRecord {
        state: JobState::Completed,
        ..valid_failed
    };
    assert!(matches!(
        monitor.can_replay(&completed),
        Err(JobsMonitorError::NotEligibleForReplay(_))
    ));
}

#[test]
fn default_monitor_summarizes_and_allows_dead_letter_replay() {
    let monitor = JobsMonitor::default();
    let summary = monitor.summarize(1, 2, 3, 4, Some(Utc::now()));
    assert_eq!(summary.pending_count, 1);
    assert_eq!(summary.failed_count, 2);
    assert_eq!(summary.dead_letter_count, 3);
    assert_eq!(summary.unprocessed_events_count, 4);
    assert!(summary.last_processed_at.is_some());

    let dead = FailedEventRecord {
        event_id: Uuid::new_v4(),
        event_type: "billing.invoice.paid".into(),
        correlation_id: Uuid::new_v4(),
        attempt_count: 1,
        last_error: "provider timeout".into(),
        state: JobState::DeadLetter,
        failed_at: Utc::now(),
    };
    assert!(monitor.can_replay(&dead).is_ok());

    let encoded = serde_json::to_string(&JobState::Pending).unwrap();
    assert_eq!(encoded, "\"pending\"");
}
