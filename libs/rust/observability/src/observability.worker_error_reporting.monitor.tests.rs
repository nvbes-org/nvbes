use super::{
    ErrorReportingSmokeResult, WorkerMonitorSchedule, capture_error_reporting_smoke,
    capture_worker_heartbeat, monitor_config, start_worker_monitor_check_in, worker_monitor_slug,
};
use crate::error_reporting::{ErrorReportingConfig, init_error_reporting_with_config};
use sentry::protocol::{MonitorIntervalUnit, MonitorSchedule};

#[test]
fn worker_monitor_slug_normalizes_to_stable_ascii_slug() {
    assert_eq!(
        worker_monitor_slug("Nvbes Account Worker", "housekeeping.expired accounts"),
        "nvbes-account-worker-housekeeping-expired-accounts"
    );
}

#[test]
fn worker_monitor_slug_collapses_repeated_separators() {
    assert_eq!(
        worker_monitor_slug("cloud_worker", "loop::heartbeat"),
        "cloud-worker-loop-heartbeat"
    );
}

#[test]
fn worker_monitor_slug_trims_leading_and_trailing_separators() {
    assert_eq!(worker_monitor_slug("---app---", "***task***"), "app-task");
    assert_eq!(worker_monitor_slug("app", "task"), "app-task");
}

#[test]
fn error_reporting_smoke_result_serializes_with_snake_case_contract() {
    let result = ErrorReportingSmokeResult {
        status: "skipped",
        app_name: "cloud-service".to_string(),
        environment: "test".to_string(),
        runtime: "api".to_string(),
        event_id: uuid::Uuid::nil().to_string(),
        check_in_id: uuid::Uuid::nil().to_string(),
        monitor_slug: "cloud-service-error-reporting-smoke".to_string(),
        configured: false,
        flushed: false,
    };

    let serialized = serde_json::to_value(result).expect("smoke result must serialize");
    assert_eq!(
        serialized["monitor_slug"],
        "cloud-service-error-reporting-smoke"
    );
    assert_eq!(serialized["status"], "skipped");
    assert_eq!(serialized["configured"], false);
}

#[test]
fn worker_monitor_config_uses_minute_interval_schedule() {
    let config = monitor_config(WorkerMonitorSchedule {
        interval_minutes: 5,
        checkin_margin_minutes: 2,
        max_runtime_minutes: 1,
    });

    assert_eq!(
        config.schedule,
        MonitorSchedule::Interval {
            value: 5,
            unit: MonitorIntervalUnit::Minute
        }
    );
    assert_eq!(config.checkin_margin, Some(2));
    assert_eq!(config.max_runtime, Some(1));
    assert_eq!(config.timezone.as_deref(), Some("UTC"));
}

#[test]
fn worker_monitor_config_clamps_zero_interval_to_one_minute() {
    let config = monitor_config(WorkerMonitorSchedule {
        interval_minutes: 0,
        checkin_margin_minutes: 1,
        max_runtime_minutes: 1,
    });
    assert_eq!(
        config.schedule,
        MonitorSchedule::Interval {
            value: 1,
            unit: MonitorIntervalUnit::Minute
        }
    );
}

#[test]
fn smoke_and_heartbeat_are_safe_when_error_reporting_is_disabled() {
    let result = capture_error_reporting_smoke("nvbes-observability", "test", "unit", false);
    assert_eq!(result.status, "skipped");
    assert!(!result.configured);
    assert!(!result.flushed);
    assert_eq!(
        result.monitor_slug,
        "nvbes-observability-error-reporting-smoke"
    );

    let schedule = WorkerMonitorSchedule {
        interval_minutes: 10,
        checkin_margin_minutes: 2,
        max_runtime_minutes: 1,
    };
    capture_worker_heartbeat("test", "nvbes-observability-heartbeat", schedule);
    let check_in = start_worker_monitor_check_in("test", "nvbes-observability-job", schedule);
    check_in.finish_ok();

    let check_in = start_worker_monitor_check_in("test", "nvbes-observability-job", schedule);
    check_in.finish_error();
}

#[test]
fn smoke_and_heartbeat_send_when_error_reporting_is_configured() {
    let _guard = init_error_reporting_with_config(ErrorReportingConfig {
        app_name: "nvbes-observability-tests",
        service_name: "nvbes-observability-tests",
        environment: "test",
        dsn: Some("https://public@127.0.0.1/1"),
        traces_sample_rate: 0.0,
    });

    let result = capture_error_reporting_smoke("nvbes-observability", "test", "unit", true);
    assert_eq!(result.status, "sent");
    assert!(result.configured);

    let schedule = WorkerMonitorSchedule {
        interval_minutes: 10,
        checkin_margin_minutes: 2,
        max_runtime_minutes: 1,
    };
    capture_worker_heartbeat("test", "nvbes-observability-heartbeat", schedule);
    let check_in = start_worker_monitor_check_in("test", "nvbes-observability-job", schedule);
    check_in.finish_ok();
    let check_in = start_worker_monitor_check_in("test", "nvbes-observability-job", schedule);
    check_in.finish_error();
}
