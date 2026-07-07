use crate::{
    domains::enterprise::types::{EnterpriseMfaPolicy, EnterpriseSessionPolicy},
    http::error::AppError,
};

pub(super) fn session_policy_view(
    policy_ttl_hours: Option<i64>,
    fallback_ttl_hours: i64,
) -> EnterpriseSessionPolicy {
    let admin_session_ttl_hours = policy_ttl_hours.unwrap_or(fallback_ttl_hours);
    EnterpriseSessionPolicy {
        admin_session_ttl_hours,
        recommended_admin_session_ttl_hours: 8,
        compliant: admin_session_ttl_hours <= 8,
        step_up_required_for_admin_elevation: true,
        source: if policy_ttl_hours.is_some() {
            "tenant_policy".to_string()
        } else {
            "environment".to_string()
        },
    }
}

pub(super) fn mfa_policy_view(policy: &str) -> EnterpriseMfaPolicy {
    EnterpriseMfaPolicy {
        compliant: policy == "required_admins" || policy == "required_all",
        policy: policy.to_string(),
        recommended_policy: "required_admins".to_string(),
    }
}

pub(super) fn session_policy_ttl(
    policy_set: &[crate::grpc_pb::nvbes::enterprise::v1::EnterprisePolicy],
) -> Result<Option<i64>, AppError> {
    let Some(policy) = policy_set
        .iter()
        .find(|policy| policy.policy_kind == "session")
    else {
        return Ok(None);
    };
    let rules = serde_json::from_str::<serde_json::Value>(&policy.rules_json).map_err(|error| {
        AppError::internal(
            "enterprise_grpc_invalid_policy_set",
            format!("Enterprise gRPC returned invalid session policy JSON: {error}"),
        )
    })?;
    Ok(rules
        .get("admin_session_ttl_hours")
        .and_then(serde_json::Value::as_i64))
}

pub(super) fn mfa_policy_value(
    policy_set: &[crate::grpc_pb::nvbes::enterprise::v1::EnterprisePolicy],
) -> Result<String, AppError> {
    let Some(policy) = policy_set.iter().find(|policy| policy.policy_kind == "mfa") else {
        return Ok("optional".to_string());
    };
    let rules = policy.rules_json.trim();
    if rules.starts_with('{') {
        let value = serde_json::from_str::<serde_json::Value>(rules).map_err(|error| {
            AppError::internal(
                "enterprise_grpc_invalid_policy_set",
                format!("Enterprise gRPC returned invalid MFA policy JSON: {error}"),
            )
        })?;
        Ok(value
            .get("policy")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("optional")
            .to_string())
    } else if rules.is_empty() {
        Ok("optional".to_string())
    } else {
        Ok(rules.to_string())
    }
}

#[cfg(test)]
#[path = "identity.domains.enterprise.service.reads.tests.rs"]
mod tests;
