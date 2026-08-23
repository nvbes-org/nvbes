use tonic::Status;
use uuid::Uuid;

use crate::grpc::pb::nvbes::platform::v1::RequestContext;

pub fn validate_context(context: Option<&RequestContext>) -> Result<(), Status> {
    let context = context.ok_or_else(|| {
        Status::invalid_argument("request context is required for Developer gRPC")
    })?;
    if context.request_id.trim().is_empty() || context.actor_principal_id.trim().is_empty() {
        return Err(Status::invalid_argument(
            "request_id and actor_principal_id are required in request context",
        ));
    }
    Ok(())
}

pub fn parse_uuid(value: &str, field: &'static str) -> Result<Uuid, Status> {
    Uuid::parse_str(value.trim())
        .map_err(|_| Status::invalid_argument(format!("{field} must be a valid UUID")))
}

pub fn non_empty(value: String, field: &'static str) -> Result<String, Status> {
    let value = value.trim().to_string();
    if value.is_empty() {
        Err(Status::invalid_argument(format!("{field} is required")))
    } else {
        Ok(value)
    }
}

pub fn sql_status(error: sqlx::Error) -> Status {
    match error {
        sqlx::Error::RowNotFound => Status::not_found("developer resource was not found"),
        sqlx::Error::Database(error) => match error.code().as_deref() {
            Some("23505") => Status::already_exists("developer resource already exists"),
            Some("23503") => Status::failed_precondition(
                "developer command references a tenant or principal projection that is missing",
            ),
            _ => Status::internal(format!("developer database error: {error}")),
        },
        error => Status::internal(format!("developer database error: {error}")),
    }
}
