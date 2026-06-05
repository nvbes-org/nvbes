use std::time::Instant;

use sentry::protocol::{
    Envelope, MonitorCheckIn, MonitorCheckInStatus, MonitorConfig, MonitorIntervalUnit,
    MonitorSchedule, Value,
};
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
    environment: Option<String>,
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
pub struct SentrySmokeResult {
    pub status: &'static str,
    pub app_name: String,
    pub environment: String,
    pub runtime: String,
    pub event_id: String,
    pub check_in_id: String,
    pub monitor_slug: String,
    pub dsn_configured: bool,
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

pub fn capture_sentry_smoke(
    app_name: &str,
    environment: &str,
    runtime: &str,
    dsn_configured: bool,
) -> SentrySmokeResult {
    let monitor_slug = worker_monitor_slug(app_name, "sentry-smoke");
    let check_in_id = Uuid::new_v4();
    let event_id = sentry::with_scope(
        |scope| {
            scope.set_transaction(Some("observability.sentry_smoke"));
            scope.set_tag("app", app_name);
            scope.set_tag("environment", environment);
            scope.set_tag("runtime", runtime);
            scope.set_tag("smoke_test", "sentry");
        },
        || sentry::capture_message("nvbes sentry smoke test", sentry::Level::Info),
    );

    let check_in_sent = send_check_in(
        check_in_id,
        &monitor_slug,
        MonitorCheckInStatus::Ok,
        Some(environment),
        Some(0.0),
        None,
    );
    let flushed = check_in_sent && flush_sentry(std::time::Duration::from_secs(2));

    SentrySmokeResult {
        status: "accepted",
        app_name: app_name.to_owned(),
        environment: environment.to_owned(),
        runtime: runtime.to_owned(),
        event_id: event_id.to_string(),
        check_in_id: check_in_id.to_string(),
        monitor_slug,
        dsn_configured,
        flushed,
    }
}

pub fn capture_worker_heartbeat(
    environment: &str,
    monitor_slug: &str,
    schedule: WorkerMonitorSchedule,
) {
    send_check_in(
        Uuid::new_v4(),
        monitor_slug,
        MonitorCheckInStatus::Ok,
        Some(environment),
        None,
        Some(monitor_config(schedule)),
    );
}

pub fn start_worker_monitor_check_in(
    environment: &str,
    monitor_slug: &str,
    schedule: WorkerMonitorSchedule,
) -> WorkerMonitorCheckIn {
    let check_in = WorkerMonitorCheckIn {
        check_in_id: Uuid::new_v4(),
        monitor_slug: monitor_slug.to_owned(),
        environment: Some(environment.to_owned()),
        started_at: Instant::now(),
    };

    send_check_in(
        check_in.check_in_id,
        &check_in.monitor_slug,
        MonitorCheckInStatus::InProgress,
        check_in.environment.as_deref(),
        None,
        Some(monitor_config(schedule)),
    );

    check_in
}

impl WorkerMonitorCheckIn {
    pub fn finish_ok(self) {
        self.finish(MonitorCheckInStatus::Ok);
    }

    pub fn finish_error(self) {
        self.finish(MonitorCheckInStatus::Error);
    }

    fn finish(self, status: MonitorCheckInStatus) {
        send_check_in(
            self.check_in_id,
            &self.monitor_slug,
            status,
            self.environment.as_deref(),
            Some(self.started_at.elapsed().as_secs_f64()),
            None,
        );
    }
}

pub fn capture_worker_job_error(
    error: &(dyn std::error::Error + Send + Sync + 'static),
    context: &WorkerJobContext<'_>,
) {
    sentry::with_scope(
        |scope| {
            scope.set_transaction(Some("worker.job"));
            scope.set_tag("app", context.app_name);
            scope.set_tag("environment", context.environment);
            scope.set_tag("worker.queue", context.queue);
            scope.set_tag("worker.job_type", context.job_type);
            scope.set_tag("worker.job_id", context.job_id);
            scope.set_extra("worker.attempts", Value::from(context.attempts));
            scope.set_extra("worker.max_attempts", Value::from(context.max_attempts));
        },
        || sentry::capture_error(error),
    );
}

fn monitor_config(schedule: WorkerMonitorSchedule) -> MonitorConfig {
    MonitorConfig {
        schedule: MonitorSchedule::Interval {
            value: schedule.interval_minutes,
            unit: MonitorIntervalUnit::Minute,
        },
        checkin_margin: Some(schedule.checkin_margin_minutes),
        max_runtime: Some(schedule.max_runtime_minutes),
        timezone: Some("UTC".to_string()),
        failure_issue_threshold: Some(1),
        recovery_threshold: Some(1),
    }
}

fn send_check_in(
    check_in_id: Uuid,
    monitor_slug: &str,
    status: MonitorCheckInStatus,
    environment: Option<&str>,
    duration: Option<f64>,
    monitor_config: Option<MonitorConfig>,
) -> bool {
    sentry::Hub::with_active(|hub| {
        let Some(client) = hub.client() else {
            return false;
        };

        let mut envelope = Envelope::new();
        envelope.add_item(MonitorCheckIn {
            check_in_id,
            monitor_slug: monitor_slug.to_owned(),
            status,
            environment: environment.map(ToOwned::to_owned),
            duration,
            monitor_config,
        });
        client.send_envelope(envelope);
        true
    })
}

fn flush_sentry(timeout: std::time::Duration) -> bool {
    sentry::Hub::with_active(|hub| match hub.client() {
        Some(client) => client.flush(Some(timeout)),
        None => false,
    })
}

#[cfg(test)]
mod tests {
    use super::{SentrySmokeResult, worker_monitor_slug};

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
    fn sentry_smoke_result_serializes_with_snake_case_contract() {
        let result = SentrySmokeResult {
            status: "accepted",
            app_name: "drive-api".to_string(),
            environment: "test".to_string(),
            runtime: "api".to_string(),
            event_id: uuid::Uuid::nil().to_string(),
            check_in_id: uuid::Uuid::nil().to_string(),
            monitor_slug: "drive-api-sentry-smoke".to_string(),
            dsn_configured: true,
            flushed: true,
        };

        let serialized = serde_json::to_value(result).expect("smoke result must serialize");

        assert_eq!(serialized["monitor_slug"], "drive-api-sentry-smoke");
    }
}
