use crate::backoffice_authorization::BackofficePermission;

pub(crate) fn record_action_request(
    family: &'static str,
    method: &str,
    policy: &'static str,
    status: u16,
    duration_seconds: f64,
) {
    let labels = [
        ("family", family.to_string()),
        ("method", method.to_string()),
        ("policy", policy.to_string()),
        ("status", status.to_string()),
        ("outcome", outcome_for_status(status).to_string()),
    ];
    metrics::counter!("internal_admin_action_requests_total", &labels).increment(1);
    metrics::histogram!("internal_admin_action_request_duration_seconds", &labels)
        .record(duration_seconds);
}

pub(crate) fn record_guard_rejection(guard: &'static str, reason: &'static str) {
    let labels = [("guard", guard.to_string()), ("reason", reason.to_string())];
    metrics::counter!("internal_admin_guard_rejections_total", &labels).increment(1);
    tracing::warn!(guard, reason, "internal admin guard rejected request");
}

pub(crate) fn record_permission_denied(permission: BackofficePermission, role: &'static str) {
    let labels = [
        ("permission", permission.metric_name().to_string()),
        ("role", role.to_string()),
    ];
    metrics::counter!("internal_admin_permission_denied_total", &labels).increment(1);
    tracing::warn!(
        permission = permission.metric_name(),
        role,
        "internal admin permission denied"
    );
}

pub(crate) fn record_rate_limited(policy: &'static str, partition: &'static str) {
    let labels = [
        ("policy", policy.to_string()),
        ("partition", partition.to_string()),
    ];
    metrics::counter!("internal_admin_rate_limited_total", &labels).increment(1);
    tracing::warn!(policy, partition, "internal admin rate limit exceeded");
}

fn outcome_for_status(status: u16) -> &'static str {
    match status {
        200..=399 => "success",
        400..=499 => "rejected",
        _ => "failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_outcome_groups_operator_rejections_and_failures() {
        assert_eq!(outcome_for_status(204), "success");
        assert_eq!(outcome_for_status(403), "rejected");
        assert_eq!(outcome_for_status(500), "failed");
    }
}
