use std::collections::HashMap;

use nvbes_core::authz::{
    ResourceContext, WorkspaceRole, action_requires_independent_approval, action_requires_step_up,
    is_allowed, parse_action, parse_role,
};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::enterprise::v1 as enterprise,
    policies,
    policy_constraints::{AuthorizationConstraints, AuthorizationEnvironment, DeviceTrust},
    service_status::sql_status,
};

pub async fn evaluate_policy(
    db: &sqlx::PgPool,
    request: enterprise::EvaluatePolicyRequest,
) -> Result<enterprise::PolicyDecision, Status> {
    let tenant_id = Uuid::parse_str(&request.tenant_id)
        .map_err(|_| Status::invalid_argument("tenant_id must be a UUID"))?;
    let principal_id = Uuid::parse_str(&request.subject_principal_id)
        .map_err(|_| Status::invalid_argument("subject_principal_id must be a UUID"))?;
    let canonical_action = parse_action(&request.action)
        .ok_or_else(|| Status::invalid_argument("action is not supported"))?
        .as_str();
    let workspace_id = optional_uuid_attribute(&request.attributes, "workspace_id")?;
    let organization_id = optional_uuid_attribute(&request.attributes, "organization_id")?;
    let constraints =
        policies::effective_authorization_constraints(db, tenant_id, organization_id, workspace_id)
            .await?;
    let elevated_role = active_elevated_role(db, tenant_id, principal_id).await?;
    let has_approval = has_independent_approval(
        db,
        tenant_id,
        principal_id,
        canonical_action,
        &request.resource,
    )
    .await?;

    evaluate_with_context(request, &constraints, elevated_role, has_approval)
}

pub fn evaluate_with_context(
    request: enterprise::EvaluatePolicyRequest,
    constraints: &AuthorizationConstraints,
    elevated_role: Option<WorkspaceRole>,
    has_independent_approval: bool,
) -> Result<enterprise::PolicyDecision, Status> {
    let action = parse_action(&request.action)
        .ok_or_else(|| Status::invalid_argument("action is not supported"))?;
    let workspace_id = parse_uuid_attribute(&request.attributes, "workspace_id")?;
    let subject_type = required_attribute(&request.attributes, "subject_type")?;
    let subject_id = required_attribute(&request.attributes, "subject_id")?;
    let subject_label = required_attribute(&request.attributes, "subject_label")?;
    let request_subject_id = Uuid::parse_str(&request.subject_principal_id)
        .map_err(|_| Status::invalid_argument("subject_principal_id must be a UUID"))?;
    if Uuid::parse_str(subject_id)
        .map_err(|_| Status::invalid_argument("subject_id attribute must be a UUID"))?
        != request_subject_id
    {
        return Err(Status::invalid_argument(
            "subject_id must match subject_principal_id",
        ));
    }

    let email_verified = bool_attribute(&request.attributes, "email_verified")?;
    let role = elevated_role.or(optional_role(&request.attributes)?);
    let resource = ResourceContext {
        owns_resource: bool_attribute(&request.attributes, "owns_resource")?,
        member_share_links_enabled: bool_attribute(
            &request.attributes,
            "member_share_links_enabled",
        )?,
        target_role: optional_target_role(&request.attributes)?,
    };
    let environment = authorization_environment(&request.attributes, subject_type)?;

    let (allowed, reason, requires_step_up) = match role {
        None => (false, "workspace_access_denied", false),
        Some(_) if subject_type == "user" && !email_verified => {
            (false, "email_not_verified", false)
        }
        Some(role) if !is_allowed(role, action, resource) => (false, "permission_denied", false),
        Some(_) if action_requires_independent_approval(action) && !has_independent_approval => {
            (false, "independent_approval_required", true)
        }
        Some(_) => match constraints.denial_reason(action.as_str(), &environment) {
            Some(reason) => (false, reason, reason == "step_up_required_by_policy"),
            None => (
                true,
                "allowed",
                subject_type == "user"
                    && (action_requires_step_up(action) || constraints.require_step_up),
            ),
        },
    };

    let role = role.map(role_label).unwrap_or_default();
    Ok(enterprise::PolicyDecision {
        decision_id: Uuid::new_v4().to_string(),
        result: if allowed { "allow" } else { "deny" }.to_string(),
        obligations: vec![
            format!("workspace_id={workspace_id}"),
            format!("subject_id={subject_id}"),
            format!("subject_label={subject_label}"),
            format!("device_trust={:?}", environment.device_trust).to_lowercase(),
            format!("data_region={}", environment.data_region),
        ],
        reason: reason.to_string(),
        requires_step_up,
        role,
    })
}

async fn active_elevated_role(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    principal_id: Uuid,
) -> Result<Option<WorkspaceRole>, Status> {
    let role = sqlx::query_scalar::<_, String>(
        r#"
        SELECT role::text
        FROM privileged_access_grants
        WHERE tenant_id = $1
          AND principal_id = $2
          AND status = 'active'
          AND revoked_at IS NULL
          AND expires_at > NOW()
        ORDER BY expires_at DESC
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .fetch_optional(db)
    .await
    .map_err(sql_status)?;

    role.map(|value| {
        parse_role(&value).ok_or_else(|| Status::internal("stored elevated role is unsupported"))
    })
    .transpose()
}

async fn has_independent_approval(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    principal_id: Uuid,
    action: &str,
    resource: &str,
) -> Result<bool, Status> {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM privileged_action_approvals
          WHERE tenant_id = $1
            AND requested_by = $2
            AND approved_by <> $2
            AND action = $3
            AND resource = $4
            AND consumed_at IS NULL
            AND expires_at > NOW()
        )
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(action)
    .bind(resource)
    .fetch_one(db)
    .await
    .map_err(sql_status)
}

fn authorization_environment(
    attributes: &HashMap<String, String>,
    subject_type: &str,
) -> Result<AuthorizationEnvironment, Status> {
    let device_trust = match optional_attribute(attributes, "device_trust").unwrap_or("unknown") {
        "unknown" => DeviceTrust::Unknown,
        "registered" => DeviceTrust::Registered,
        "managed" => DeviceTrust::Managed,
        "hardware_attested" => DeviceTrust::HardwareAttested,
        _ => {
            return Err(Status::invalid_argument(
                "device_trust attribute is not supported",
            ));
        }
    };
    let risk_score = optional_attribute(attributes, "risk_score")
        .unwrap_or("100")
        .parse::<u8>()
        .map_err(|_| Status::invalid_argument("risk_score must be between 0 and 100"))?;
    if risk_score > 100 {
        return Err(Status::invalid_argument(
            "risk_score must be between 0 and 100",
        ));
    }

    Ok(AuthorizationEnvironment {
        device_trust,
        network_zone: optional_attribute(attributes, "network_zone")
            .unwrap_or("unknown")
            .to_string(),
        risk_score,
        data_region: optional_attribute(attributes, "data_region")
            .unwrap_or("unknown")
            .to_string(),
        subject_type: subject_type.to_string(),
        step_up_active: optional_bool_attribute(attributes, "step_up_active")?.unwrap_or(false),
    })
}

fn required_attribute<'a>(
    attributes: &'a HashMap<String, String>,
    key: &str,
) -> Result<&'a str, Status> {
    optional_attribute(attributes, key)
        .ok_or_else(|| Status::invalid_argument(format!("{key} attribute is required")))
}

fn optional_attribute<'a>(attributes: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
    attributes
        .get(key)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
}

fn parse_uuid_attribute(attributes: &HashMap<String, String>, key: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(required_attribute(attributes, key)?)
        .map_err(|_| Status::invalid_argument(format!("{key} attribute must be a UUID")))
}

fn optional_uuid_attribute(
    attributes: &HashMap<String, String>,
    key: &str,
) -> Result<Option<Uuid>, Status> {
    optional_attribute(attributes, key)
        .map(|value| {
            Uuid::parse_str(value)
                .map_err(|_| Status::invalid_argument(format!("{key} attribute must be a UUID")))
        })
        .transpose()
}

fn bool_attribute(attributes: &HashMap<String, String>, key: &str) -> Result<bool, Status> {
    optional_bool_attribute(attributes, key)?
        .ok_or_else(|| Status::invalid_argument(format!("{key} attribute is required")))
}

fn optional_bool_attribute(
    attributes: &HashMap<String, String>,
    key: &str,
) -> Result<Option<bool>, Status> {
    match optional_attribute(attributes, key) {
        None => Ok(None),
        Some("true") => Ok(Some(true)),
        Some("false") => Ok(Some(false)),
        Some(_) => Err(Status::invalid_argument(format!(
            "{key} attribute must be true or false"
        ))),
    }
}

fn optional_role(attributes: &HashMap<String, String>) -> Result<Option<WorkspaceRole>, Status> {
    optional_parsed_role(attributes, "role")
}

fn optional_target_role(
    attributes: &HashMap<String, String>,
) -> Result<Option<WorkspaceRole>, Status> {
    optional_parsed_role(attributes, "target_role")
}

fn optional_parsed_role(
    attributes: &HashMap<String, String>,
    key: &str,
) -> Result<Option<WorkspaceRole>, Status> {
    optional_attribute(attributes, key)
        .map(|role| {
            parse_role(role)
                .ok_or_else(|| Status::invalid_argument(format!("{key} attribute is unsupported")))
        })
        .transpose()
}

fn role_label(role: WorkspaceRole) -> String {
    role.to_string()
}
