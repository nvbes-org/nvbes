use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    grpc::pb::nvbes::platform::v1::{RequestContext, TenantContext},
    http::{auth::DeveloperAuth, error::AppError},
};

pub fn request_context(auth: &DeveloperAuth) -> RequestContext {
    let request_id = Uuid::new_v4().to_string();
    RequestContext {
        request_id: request_id.clone(),
        correlation_id: request_id,
        actor_principal_id: auth.user_id.to_string(),
        tenant: Some(TenantContext {
            tenant_id: auth.tenant_id.to_string(),
            workspace_id: auth
                .claims
                .workspace_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            region_id: String::new(),
            data_residency: String::new(),
        }),
    }
}

pub fn uuid(value: &str, field: &'static str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| {
        AppError::internal(
            "developer_contract_invalid",
            format!("Developer returned an invalid {field}."),
        )
    })
}

pub fn optional_uuid(value: &str, field: &'static str) -> Result<Option<Uuid>, AppError> {
    if value.is_empty() {
        Ok(None)
    } else {
        uuid(value, field).map(Some)
    }
}

pub fn time(value: &str, field: &'static str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| {
            AppError::internal(
                "developer_contract_invalid",
                format!("Developer returned an invalid {field}."),
            )
        })
}

pub fn optional_time(value: &str, field: &'static str) -> Result<Option<DateTime<Utc>>, AppError> {
    if value.is_empty() {
        Ok(None)
    } else {
        time(value, field).map(Some)
    }
}

pub fn optional_string(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}
