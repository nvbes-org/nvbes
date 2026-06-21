use std::time::Instant;

use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct WorkerMonitorSchedule {
    pub interval_minutes: u64,
    pub checkin_margin_minutes: u64,
    pub max_runtime_minutes: u64,
}

#[derive(Debug)]
pub struct WorkerMonitorCheckIn {
    check_in_id: Uuid,
    monitor_slug: String,
    started_at: Instant,
}

#[derive(Debug)]
pub struct WorkerJobContext<'a> {
    pub app_name: &'a str,
    pub environment: &'a str,
    pub queue: &'a str,
    pub job_type: &'a str,
    pub job_id: Uuid,
    pub attempts: u32,
    pub max_attempts: u32,
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

    ErrorReportingSmokeResult {
        status: "noop",
        app_name: app_name.to_owned(),
        environment: environment.to_owned(),
        runtime: runtime.to_owned(),
        event_id: Uuid::new_v4().to_string(),
        check_in_id: Uuid::new_v4().to_string(),
        monitor_slug,
        configured,
        flushed: false,
    }
}

pub fn capture_worker_heartbeat(
    environment: &str,
    monitor_slug: &str,
    schedule: WorkerMonitorSchedule,
) {
    let _ = (environment, monitor_slug, schedule);
}

pub fn start_worker_monitor_check_in(
    environment: &str,
    monitor_slug: &str,
    schedule: WorkerMonitorSchedule,
) -> WorkerMonitorCheckIn {
    let _ = (environment, schedule);

    WorkerMonitorCheckIn {
        check_in_id: Uuid::new_v4(),
        monitor_slug: monitor_slug.to_owned(),
        started_at: Instant::now(),
    }
}

impl WorkerMonitorCheckIn {
    pub fn finish_ok(self) {
        self.finish("ok");
    }

    pub fn finish_error(self) {
        self.finish("error");
    }

    fn finish(self, status: &'static str) {
        let _ = (
            self.check_in_id,
            self.monitor_slug,
            self.started_at.elapsed(),
            status,
        );
    }
}

pub fn capture_worker_job_error(
    error: &(dyn std::error::Error + Send + Sync + 'static),
    context: &WorkerJobContext<'_>,
) {
    let _ = error;
    tracing::debug!(
        app = context.app_name,
        environment = context.environment,
        queue = context.queue,
        job_type = context.job_type,
        job_id = %context.job_id,
        attempts = context.attempts,
        max_attempts = context.max_attempts,
        "worker job error observed"
    );
}

#[cfg(test)]
mod tests {
    use super::{ErrorReportingSmokeResult, worker_monitor_slug};

    #[test]
    fn worker_monitor_slug_normalizes_to_stable_ascii_slug() {
        assert_eq!(
            worker_monitor_slug("Nvbes Identity Worker", "housekeeping.expired accounts"),
            "nvbes-identity-worker-housekeeping-expired-accounts"
        );
    }

    #[test]
    fn worker_monitor_slug_collapses_repeated_separators() {
        assert_eq!(
            worker_monitor_slug("drive_worker", "loop::heartbeat"),
            "drive-worker-loop-heartbeat"
        );
    }

    #[test]
    fn error_reporting_smoke_result_serializes_with_snake_case_contract() {
        let result = ErrorReportingSmokeResult {
            status: "noop",
            app_name: "drive-api".to_string(),
            environment: "test".to_string(),
            runtime: "api".to_string(),
            event_id: uuid::Uuid::nil().to_string(),
            check_in_id: uuid::Uuid::nil().to_string(),
            monitor_slug: "drive-api-error-reporting-smoke".to_string(),
            configured: false,
            flushed: false,
        };

        let serialized = serde_json::to_value(result).expect("smoke result must serialize");

        assert_eq!(
            serialized["monitor_slug"],
            "drive-api-error-reporting-smoke"
        );
    }
}
