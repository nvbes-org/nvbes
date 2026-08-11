use nvbes_observability::metrics::HttpMetrics;
use nvbes_redis::worker_queue::QueueStatusEntry;
use std::time::Instant;

use super::{QUEUE_STATUSES, REFRESH_INTERVAL, record_queue_statuses, refresh_if_due};
use crate::worker::{job_processor::WORKER_QUEUES, test_support::app_state};

#[test]
fn queue_status_metrics_fill_missing_states_with_zero() {
    let metrics = HttpMetrics::default();
    record_queue_statuses(
        &metrics,
        "integration.email.submit",
        &[
            QueueStatusEntry {
                status: "pending".to_string(),
                depth: 3,
                oldest_age_seconds: Some(12.0),
            },
            QueueStatusEntry {
                status: "dead_letter".to_string(),
                depth: 1,
                oldest_age_seconds: None,
            },
        ],
    );

    let rendered = metrics.render();
    for status in QUEUE_STATUSES {
        assert!(
            rendered.contains(&format!("status=\"{status}\"")),
            "missing {status}"
        );
    }
    assert!(rendered.contains("integration.email.submit"));
    assert_eq!(REFRESH_INTERVAL.as_secs(), 15);
}

#[tokio::test]
async fn refresh_if_due_skips_fresh_metrics_and_refreshes_all_owned_queues() {
    let state = app_state().await;
    let mut fresh = Instant::now();
    refresh_if_due(&state, &WORKER_QUEUES, &mut fresh)
        .await
        .expect("fresh no-op");

    let mut due = Instant::now() - REFRESH_INTERVAL;
    refresh_if_due(&state, &WORKER_QUEUES, &mut due)
        .await
        .expect("metrics refresh");

    assert!(due.elapsed() < REFRESH_INTERVAL);
    let rendered = state.observability.render();
    assert!(rendered.contains("integration.email.submit"));
    assert!(rendered.contains("account_registration_projection"));
}
