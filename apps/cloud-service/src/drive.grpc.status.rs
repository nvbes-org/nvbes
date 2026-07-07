use chrono::{DateTime, Utc};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::pb::nvbes::platform::v1::RequestContext;

pub fn validate_context(
    context: Option<&RequestContext>,
    workspace_id: Option<Uuid>,
) -> Result<&RequestContext, Status> {
    let context = context
        .ok_or_else(|| Status::invalid_argument("request context is required for Cloud gRPC"))?;
    if context.request_id.trim().is_empty() || context.actor_principal_id.trim().is_empty() {
        return Err(Status::invalid_argument(
            "request_id and actor_principal_id are required in request context",
        ));
    }

    if let Some(workspace_id) = workspace_id {
        let tenant = context
            .tenant
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("tenant context is required"))?;
        if !tenant.workspace_id.trim().is_empty() && tenant.workspace_id != workspace_id.to_string()
        {
            return Err(Status::permission_denied(
                "request context workspace does not match the Cloud workspace",
            ));
        }
    }

    Ok(context)
}

pub fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, Status> {
    Uuid::parse_str(value.trim())
        .map_err(|_| Status::invalid_argument(format!("{field} must be a valid UUID")))
}

pub fn optional_uuid(value: &str, field: &'static str) -> Result<Option<Uuid>, Status> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_uuid(value, field).map(Some)
    }
}

pub fn parse_datetime(value: &str, field: &'static str) -> Result<DateTime<Utc>, Status> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| Status::invalid_argument(format!("{field} must be an RFC3339 timestamp")))
}

pub fn optional_datetime(
    value: &str,
    field: &'static str,
) -> Result<Option<DateTime<Utc>>, Status> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        parse_datetime(value, field).map(Some)
    }
}

pub fn non_empty(value: String, field: &'static str) -> Result<String, Status> {
    let value = value.trim().to_string();
    if value.is_empty() {
        Err(Status::invalid_argument(format!("{field} is required")))
    } else {
        Ok(value)
    }
}

pub fn optional_string(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

pub fn sql_status(error: sqlx::Error) -> Status {
    let sqlx::Error::Database(error) = &error else {
        return Status::internal(format!("cloud database error: {error}"));
    };

    match error.code().as_deref() {
        Some("23503") => Status::failed_precondition(
            "cloud command references a principal or workspace that Cloud has not synced",
        ),
        Some("23505") => Status::already_exists("cloud resource already exists"),
        Some("22P02") => Status::invalid_argument("cloud command contains an invalid enum value"),
        _ => Status::internal(format!("cloud database error: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::validate_context;
    use crate::grpc::pb::nvbes::platform::v1::{RequestContext, TenantContext};
    use uuid::Uuid;

    #[test]
    fn context_workspace_must_match_cloud_workspace() {
        let workspace_id = Uuid::new_v4();
        let context = RequestContext {
            request_id: "req_cloud".to_string(),
            correlation_id: "corr_cloud".to_string(),
            actor_principal_id: Uuid::new_v4().to_string(),
            tenant: Some(TenantContext {
                tenant_id: Uuid::new_v4().to_string(),
                workspace_id: workspace_id.to_string(),
                region_id: "eu".to_string(),
                data_residency: "eu".to_string(),
            }),
        };

        assert!(validate_context(Some(&context), Some(workspace_id)).is_ok());
        assert!(validate_context(Some(&context), Some(Uuid::new_v4())).is_err());
    }
}
