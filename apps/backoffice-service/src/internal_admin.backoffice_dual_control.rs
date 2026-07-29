use axum::http::HeaderMap;
use uuid::Uuid;

use crate::error::AppError;
use crate::observability::record_guard_rejection;

const ACTOR_HEADER: &str = "x-nvbes-actor-principal-id";
const SECOND_APPROVER_PRINCIPAL_HEADER: &str = "x-nvbes-second-approver-principal-id";
const SECOND_APPROVER_ROLE_HEADER: &str = "x-nvbes-second-approver-role";

pub(crate) fn require_dual_control(headers: &HeaderMap) -> Result<(), AppError> {
    let actor_id = header_uuid(headers, ACTOR_HEADER, "invalid_backoffice_actor")?;
    let approver_id = header_uuid(
        headers,
        SECOND_APPROVER_PRINCIPAL_HEADER,
        "invalid_second_approver",
    )?;
    if actor_id == approver_id {
        record_guard_rejection("dual_control", "same_approver");
        return Err(AppError::forbidden(
            "second_approver_must_differ",
            "Critical back-office actions require a distinct second approver.",
        ));
    }
    let approver_role = headers
        .get(SECOND_APPROVER_ROLE_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            record_guard_rejection("dual_control", "missing_second_approver_role");
            AppError::bad_request(
                "second_approver_role_required",
                "Critical back-office actions require x-nvbes-second-approver-role.",
            )
        })?;
    if approver_role.trim() == "platform_admin" {
        return Ok(());
    }
    record_guard_rejection("dual_control", "second_approver_role_denied");
    Err(AppError::forbidden(
        "second_approver_role_denied",
        "Critical back-office actions require a platform_admin second approver.",
    ))
}

pub(crate) fn second_approver_principal_id(headers: &HeaderMap) -> Result<Uuid, AppError> {
    header_uuid(
        headers,
        SECOND_APPROVER_PRINCIPAL_HEADER,
        "invalid_second_approver",
    )
}

fn header_uuid(
    headers: &HeaderMap,
    name: &'static str,
    code: &'static str,
) -> Result<Uuid, AppError> {
    let value = headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            record_guard_rejection("dual_control", code);
            AppError::bad_request(
                format!("{code}_required"),
                format!("Back-office request requires {name}."),
            )
        })?;
    Uuid::parse_str(value).map_err(|_| {
        record_guard_rejection("dual_control", code);
        AppError::bad_request(code, format!("Back-office header {name} must be a UUID."))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dual_control_requires_distinct_platform_admin_approver() {
        let actor_id = Uuid::new_v4();
        let approver_id = Uuid::new_v4();
        let mut headers = HeaderMap::new();
        headers.insert(ACTOR_HEADER, actor_id.to_string().parse().unwrap());
        headers.insert(
            SECOND_APPROVER_PRINCIPAL_HEADER,
            approver_id.to_string().parse().unwrap(),
        );
        headers.insert(
            SECOND_APPROVER_ROLE_HEADER,
            "platform_admin".parse().unwrap(),
        );
        assert!(require_dual_control(&headers).is_ok());

        headers.insert(
            SECOND_APPROVER_PRINCIPAL_HEADER,
            actor_id.to_string().parse().unwrap(),
        );
        assert!(require_dual_control(&headers).is_err());
    }
}
