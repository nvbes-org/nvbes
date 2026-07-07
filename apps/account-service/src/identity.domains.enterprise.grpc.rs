use axum::http::StatusCode;
use tonic::{Code, transport::Channel};
use uuid::Uuid;

use crate::{
    domains::enterprise::policy_simulation::types::EnterprisePolicySimulationDecision,
    grpc_pb::nvbes::{
        enterprise::v1::{
            self as enterprise, EvaluatePolicyRequest, GetPolicySetRequest, UpdatePolicyRequest,
            enterprise_service_client::EnterpriseServiceClient,
        },
        platform::v1::{RequestContext, TenantContext},
    },
    http::error::AppError,
};

const ENTERPRISE_GRPC_ENDPOINT_ENV: &str = "NVBES_ENTERPRISE_GRPC_ENDPOINT";

#[path = "identity.domains.enterprise.grpc.access_reviews.rs"]
pub mod access_reviews;
#[path = "identity.domains.enterprise.grpc.admin_elevation.rs"]
pub mod admin_elevation;
#[path = "identity.domains.enterprise.grpc.audit.rs"]
pub mod audit;
#[path = "identity.domains.enterprise.grpc.break_glass.rs"]
pub mod break_glass;
#[path = "identity.domains.enterprise.grpc.federation.rs"]
pub mod federation;
#[path = "identity.domains.enterprise.grpc.invitations.rs"]
pub mod invitations;
#[path = "identity.domains.enterprise.grpc.trust.rs"]
pub mod trust;
#[path = "identity.domains.enterprise.grpc.user_access.rs"]
pub mod user_access;

pub fn enterprise_grpc_endpoint() -> anyhow::Result<String> {
    match std::env::var(ENTERPRISE_GRPC_ENDPOINT_ENV) {
        Ok(endpoint) if !endpoint.trim().is_empty() => Ok(endpoint),
        Ok(_) => anyhow::bail!("{ENTERPRISE_GRPC_ENDPOINT_ENV} must not be empty"),
        Err(std::env::VarError::NotPresent) => Ok("http://127.0.0.1:4031".to_string()),
        Err(error) => Err(anyhow::anyhow!(
            "{ENTERPRISE_GRPC_ENDPOINT_ENV} could not be read: {error}"
        )),
    }
}

pub async fn get_policy_set(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
) -> Result<Vec<enterprise::EnterprisePolicy>, AppError> {
    let mut client = enterprise_client().await?;
    let response = client
        .get_policy_set(GetPolicySetRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
        })
        .await
        .map_err(grpc_error)?
        .into_inner();
    Ok(response.policies)
}

pub async fn update_session_policy(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    admin_session_ttl_hours: i64,
) -> Result<(), AppError> {
    let mut client = enterprise_client().await?;
    client
        .update_policy(UpdatePolicyRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            policy_id: format!("{tenant_id}:session"),
            policy_kind: "session".to_string(),
            rules_json: serde_json::json!({
                "admin_session_ttl_hours": admin_session_ttl_hours
            })
            .to_string(),
            reason: String::new(),
        })
        .await
        .map_err(grpc_error)?;
    Ok(())
}

pub async fn update_mfa_policy(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    policy: &str,
) -> Result<(), AppError> {
    let mut client = enterprise_client().await?;
    client
        .update_policy(UpdatePolicyRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            policy_id: format!("{tenant_id}:mfa"),
            policy_kind: "mfa".to_string(),
            rules_json: serde_json::json!({ "policy": policy }).to_string(),
            reason: String::new(),
        })
        .await
        .map_err(grpc_error)?;
    Ok(())
}

pub struct PolicySimulationEvaluationInput {
    pub workspace_id: Uuid,
    pub subject_principal_id: Uuid,
    pub subject_type: String,
    pub subject_id: String,
    pub subject_label: String,
    pub email_verified: bool,
    pub action: String,
    pub role: Option<String>,
    pub owns_resource: bool,
    pub member_share_links_enabled: bool,
    pub target_role: Option<String>,
}

pub async fn evaluate_policy_simulation(
    tenant_id: Uuid,
    actor_principal_id: Uuid,
    input: PolicySimulationEvaluationInput,
) -> Result<EnterprisePolicySimulationDecision, AppError> {
    let mut attributes = std::collections::HashMap::new();
    attributes.insert("workspace_id".to_string(), input.workspace_id.to_string());
    attributes.insert("subject_type".to_string(), input.subject_type.clone());
    attributes.insert("subject_id".to_string(), input.subject_id.clone());
    attributes.insert("subject_label".to_string(), input.subject_label.clone());
    attributes.insert(
        "email_verified".to_string(),
        input.email_verified.to_string(),
    );
    attributes.insert("role".to_string(), input.role.clone().unwrap_or_default());
    attributes.insert("owns_resource".to_string(), input.owns_resource.to_string());
    attributes.insert(
        "member_share_links_enabled".to_string(),
        input.member_share_links_enabled.to_string(),
    );
    attributes.insert(
        "target_role".to_string(),
        input.target_role.clone().unwrap_or_default(),
    );

    let mut client = enterprise_client().await?;
    let decision = client
        .evaluate_policy(EvaluatePolicyRequest {
            context: Some(request_context(tenant_id, actor_principal_id)),
            tenant_id: tenant_id.to_string(),
            subject_principal_id: input.subject_principal_id.to_string(),
            action: input.action.clone(),
            resource: input.workspace_id.to_string(),
            attributes,
        })
        .await
        .map_err(grpc_error)?
        .into_inner();

    Ok(EnterprisePolicySimulationDecision {
        allowed: decision.result == "allow",
        reason: decision.reason,
        action: input.action,
        workspace_id: input.workspace_id,
        subject_type: input.subject_type,
        subject_id: input.subject_id,
        subject_label: input.subject_label,
        role: empty_to_option(decision.role),
        requires_step_up: decision.requires_step_up,
    })
}

async fn enterprise_client() -> Result<EnterpriseServiceClient<Channel>, AppError> {
    let endpoint = enterprise_grpc_endpoint().map_err(|error| {
        AppError::internal("enterprise_grpc_endpoint_invalid", error.to_string())
    })?;
    EnterpriseServiceClient::connect(endpoint)
        .await
        .map_err(|error| AppError::internal("enterprise_grpc_connect_failed", error.to_string()))
}

fn request_context(tenant_id: Uuid, actor_principal_id: Uuid) -> RequestContext {
    RequestContext {
        request_id: Uuid::new_v4().to_string(),
        correlation_id: Uuid::new_v4().to_string(),
        actor_principal_id: actor_principal_id.to_string(),
        tenant: Some(TenantContext {
            tenant_id: tenant_id.to_string(),
            workspace_id: String::new(),
            region_id: String::new(),
            data_residency: String::new(),
        }),
    }
}

fn grpc_error(error: tonic::Status) -> AppError {
    let status = match error.code() {
        Code::InvalidArgument => StatusCode::BAD_REQUEST,
        Code::Unauthenticated => StatusCode::UNAUTHORIZED,
        Code::PermissionDenied => StatusCode::FORBIDDEN,
        Code::NotFound => StatusCode::NOT_FOUND,
        Code::AlreadyExists | Code::FailedPrecondition => StatusCode::CONFLICT,
        Code::Unavailable | Code::DeadlineExceeded => StatusCode::BAD_GATEWAY,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    AppError::new(status, "enterprise_grpc_error", error.message().to_string())
}

fn empty_to_option(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
