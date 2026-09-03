use chrono::Utc;
use std::collections::HashMap;

use crate::cockpit_model::{AggregateHealth, HealthStatus, RuntimeHealth, ServiceId};

const COLD_START_LATENCY_THRESHOLD_MS: u64 = 800;

#[derive(Debug, Clone)]
pub struct HealthProbeResult {
    pub service_id: ServiceId,
    pub live: bool,
    pub ready: bool,
    pub latency_ms: u64,
    pub message: Option<String>,
}

pub struct HealthAggregator {
    cold_start_threshold_ms: u64,
}

impl Default for HealthAggregator {
    fn default() -> Self {
        Self {
            cold_start_threshold_ms: COLD_START_LATENCY_THRESHOLD_MS,
        }
    }
}

impl HealthAggregator {
    pub fn new(cold_start_threshold_ms: u64) -> Self {
        Self {
            cold_start_threshold_ms,
        }
    }

    pub fn evaluate_runtime(&self, probe: HealthProbeResult) -> RuntimeHealth {
        let is_cold_start =
            probe.live && probe.ready && probe.latency_ms >= self.cold_start_threshold_ms;
        let status = if !probe.live {
            HealthStatus::Unavailable
        } else if !probe.ready {
            HealthStatus::Degraded
        } else if is_cold_start {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        let details = match &probe.message {
            Some(msg) => msg.clone(),
            None => match status {
                HealthStatus::Healthy => "Runtime operational and responsive".to_string(),
                HealthStatus::Degraded if is_cold_start => {
                    format!("Cold start detected (latency {}ms)", probe.latency_ms)
                }
                HealthStatus::Degraded => "Runtime not ready for traffic".to_string(),
                HealthStatus::Unavailable => "Runtime liveness probe failed".to_string(),
                HealthStatus::Unknown => "Status unknown".to_string(),
            },
        };

        RuntimeHealth {
            service_id: probe.service_id,
            status,
            live: probe.live,
            ready: probe.ready,
            latency_ms: probe.latency_ms,
            is_cold_start,
            details,
            checked_at: Utc::now(),
        }
    }

    pub fn aggregate(&self, probes: Vec<HealthProbeResult>) -> AggregateHealth {
        let mut runtimes = Vec::with_capacity(probes.len());
        let mut degraded_count = 0;
        let mut has_unavailable = false;

        for probe in probes {
            let runtime = self.evaluate_runtime(probe);
            match runtime.status {
                HealthStatus::Unavailable => {
                    has_unavailable = true;
                    degraded_count += 1;
                }
                HealthStatus::Degraded => {
                    degraded_count += 1;
                }
                _ => {}
            }
            runtimes.push(runtime);
        }

        let overall_status = if has_unavailable {
            HealthStatus::Unavailable
        } else if degraded_count > 0 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        AggregateHealth {
            overall_status,
            runtimes,
            degraded_count,
            evaluated_at: Utc::now(),
        }
    }

    pub fn default_probes() -> HashMap<ServiceId, HealthProbeResult> {
        let mut map = HashMap::new();
        for &service_id in &[
            ServiceId::Identity,
            ServiceId::Account,
            ServiceId::Billing,
            ServiceId::Email,
            ServiceId::TrustRisk,
        ] {
            map.insert(
                service_id,
                HealthProbeResult {
                    service_id,
                    live: true,
                    ready: true,
                    latency_ms: 45,
                    message: None,
                },
            );
        }
        map
    }
}

#[cfg(test)]
#[path = "platform.cockpit.health.tests.rs"]
mod tests;
