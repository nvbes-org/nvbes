use std::time::{Duration, Instant};

use sentry::protocol::{
    Envelope, Level, MonitorCheckIn, MonitorCheckInStatus, MonitorConfig, MonitorIntervalUnit,
    MonitorSchedule,
};
use serde::Serialize;
use uuid::Uuid;

use crate::error_reporting::{flush_error_reporting, is_error_reporting_configured};

const ERROR_REPORTING_SMOKE_FLUSH_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy)]
pub struct WorkerMonitorSchedule {
    pub interval_minutes: u64,
    pub checkin_margin_minutes: u64,
    pub max_runtime_minutes: u64,
}

#[derive(Debug)]
pub struct WorkerMonitorCheckIn {
    check_in_id: Uuid,
    environment: String,
    monitor_slug: String,
    schedule: WorkerMonitorSchedule,
    started_at: Instant,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ErrorReportingSmokeResult {
    pub status: &'static str,
    pub app_name: String,
    pub environment: String,
    pub runtime: String,
    pub event_id: String,
    pub check_in_id: String,
    pub monitor_slug: String,
    pub configured: bool,
    pub flushed: bool,
}

pub fn worker_monitor_slug(app_name: &str, task_name: &str) -> String {
    let mut slug = String::new();
    let mut previous_separator = false;

    for character in format!("{app_name}-{task_name}").chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_separator = false;
            continue;
        }

        if !previous_separator {
            slug.push('-');
            previous_separator = true;
        }
    }

    slug.trim_matches('-').to_string()
}

pub fn capture_error_reporting_smoke(
    app_name: &str,
    environment: &str,
    runtime: &str,
    configured: bool,
) -> ErrorReportingSmokeResult {
    let monitor_slug = worker_monitor_slug(app_name, "error-reporting-smoke");
    let configured = configured || is_error_reporting_configured();
    let check_in_id = Uuid::new_v4();
    let event_id = if configured {
        sentry::with_scope(
            |scope| {
                scope.set_tag("app.name", app_name);
                scope.set_tag("runtime", runtime);
                scope.set_tag("monitor_slug", &monitor_slug);
            },
            || sentry::capture_message("Sentry error reporting smoke", Level::Info),
        )
    } else {
        Uuid::nil()
    };

    if configured {
        send_monitor_check_in(
            environment,
            &monitor_slug,
            MonitorCheckInStatus::Ok,
            check_in_id,
            Some(WorkerMonitorSchedule {
                interval_minutes: 60,
                checkin_margin_minutes: 15,
                max_runtime_minutes: 5,
            }),
            None,
        );
    }

    ErrorReportingSmokeResult {
        status: if configured { "sent" } else { "skipped" },
        app_name: app_name.to_owned(),
        environment: environment.to_owned(),
        runtime: runtime.to_owned(),
        event_id: event_id.to_string(),
        check_in_id: check_in_id.to_string(),
        monitor_slug,
        configured,
        flushed: configured && flush_error_reporting(ERROR_REPORTING_SMOKE_FLUSH_TIMEOUT),
    }
}

pub fn capture_worker_heartbeat(
    environment: &str,
    monitor_slug: &str,
    schedule: WorkerMonitorSchedule,
) {
    send_monitor_check_in(
        environment,
        monitor_slug,
        MonitorCheckInStatus::Ok,
        Uuid::new_v4(),
        Some(schedule),
        None,
    );
}

pub fn start_worker_monitor_check_in(
    environment: &str,
    monitor_slug: &str,
    schedule: WorkerMonitorSchedule,
) -> WorkerMonitorCheckIn {
    let check_in_id = Uuid::new_v4();
    send_monitor_check_in(
        environment,
        monitor_slug,
        MonitorCheckInStatus::InProgress,
        check_in_id,
        Some(schedule),
        None,
    );

    WorkerMonitorCheckIn {
        check_in_id,
        environment: environment.to_owned(),
        monitor_slug: monitor_slug.to_owned(),
        schedule,
        started_at: Instant::now(),
    }
}

impl WorkerMonitorCheckIn {
    pub fn finish_ok(self) {
        self.finish(MonitorCheckInStatus::Ok);
    }

    pub fn finish_error(self) {
        self.finish(MonitorCheckInStatus::Error);
    }

    fn finish(self, status: MonitorCheckInStatus) {
        send_monitor_check_in(
            &self.environment,
            &self.monitor_slug,
            status,
            self.check_in_id,
            Some(self.schedule),
            Some(self.started_at.elapsed().as_secs_f64()),
        );
    }
}

fn send_monitor_check_in(
    environment: &str,
    monitor_slug: &str,
    status: MonitorCheckInStatus,
    check_in_id: Uuid,
    schedule: Option<WorkerMonitorSchedule>,
    duration: Option<f64>,
) -> bool {
    if !is_error_reporting_configured() {
        return false;
    }

    let mut envelope = Envelope::new();
    envelope.add_item(MonitorCheckIn {
        check_in_id,
        monitor_slug: monitor_slug.to_owned(),
        status,
        environment: Some(environment.to_owned()),
        duration,
        monitor_config: schedule.map(monitor_config),
    });

    sentry::Hub::with_active(|hub| {
        if let Some(client) = hub.client() {
            client.send_envelope(envelope);
            true
        } else {
            false
        }
    })
}

fn monitor_config(schedule: WorkerMonitorSchedule) -> MonitorConfig {
    MonitorConfig {
        schedule: MonitorSchedule::Interval {
            value: schedule.interval_minutes.max(1),
            unit: MonitorIntervalUnit::Minute,
        },
        checkin_margin: Some(schedule.checkin_margin_minutes),
        max_runtime: Some(schedule.max_runtime_minutes),
        timezone: Some("UTC".to_owned()),
        failure_issue_threshold: Some(1),
        recovery_threshold: Some(1),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ErrorReportingSmokeResult, WorkerMonitorSchedule, monitor_config, worker_monitor_slug,
    };
    use sentry::protocol::{MonitorIntervalUnit, MonitorSchedule};

    #[test]
    fn worker_monitor_slug_normalizes_to_stable_ascii_slug() {
        assert_eq!(
            worker_monitor_slug("Nvbes Identity Worker", "housekeeping.expired accounts"),
            "nvbes-account-worker-housekeeping-expired-accounts"
        );
    }

    #[test]
    fn worker_monitor_slug_collapses_repeated_separators() {
        assert_eq!(
            worker_monitor_slug("drive_worker", "loop::heartbeat"),
            "cloud-worker-loop-heartbeat"
        );
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
    }
}
