use nvbes_core::authz::{
    ResourceContext, WorkspaceRole, action_requires_step_up, is_allowed, parse_action, parse_role,
};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::pb::nvbes::enterprise::v1 as enterprise;

pub fn evaluate_policy(
    request: enterprise::EvaluatePolicyRequest,
) -> Result<enterprise::PolicyDecision, Status> {
    let action = parse_action(&request.action)
        .ok_or_else(|| Status::invalid_argument("action is not supported"))?;
    let workspace_id = parse_uuid_attribute(&request.attributes, "workspace_id")?;
    let subject_type = required_attribute(&request.attributes, "subject_type")?;
    let subject_id = required_attribute(&request.attributes, "subject_id")?;
    let subject_label = required_attribute(&request.attributes, "subject_label")?;
    let email_verified = bool_attribute(&request.attributes, "email_verified")?;
    let role = optional_role(&request.attributes)?;
    let resource = ResourceContext {
        owns_resource: bool_attribute(&request.attributes, "owns_resource")?,
        member_share_links_enabled: bool_attribute(
            &request.attributes,
            "member_share_links_enabled",
        )?,
        target_role: optional_target_role(&request.attributes)?,
    };

    let (allowed, reason, requires_step_up) = match role {
        None => (false, "workspace_access_denied", false),
        Some(_) if subject_type == "user" && !email_verified => {
            (false, "email_not_verified", false)
        }
        Some(role) if !is_allowed(role, action, resource) => (false, "permission_denied", false),
        Some(_) => (
            true,
            "allowed",
            subject_type == "user" && action_requires_step_up(action),
        ),
    };

    let role = role.map(role_label).unwrap_or_default();
    Ok(enterprise::PolicyDecision {
        decision_id: uuid::Uuid::new_v4().to_string(),
        result: if allowed { "allow" } else { "deny" }.to_string(),
        obligations: vec![
            format!("workspace_id={workspace_id}"),
            format!("subject_id={subject_id}"),
            format!("subject_label={subject_label}"),
        ],
        reason: reason.to_string(),
        requires_step_up,
        role,
    })
}

fn required_attribute<'a>(
    attributes: &'a std::collections::HashMap<String, String>,
    key: &str,
) -> Result<&'a str, Status> {
    attributes
        .get(key)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| Status::invalid_argument(format!("{key} attribute is required")))
}

fn parse_uuid_attribute(
    attributes: &std::collections::HashMap<String, String>,
    key: &str,
) -> Result<Uuid, Status> {
    Uuid::parse_str(required_attribute(attributes, key)?)
        .map_err(|_| Status::invalid_argument(format!("{key} attribute must be a UUID")))
}

fn bool_attribute(
    attributes: &std::collections::HashMap<String, String>,
    key: &str,
) -> Result<bool, Status> {
    match required_attribute(attributes, key)? {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(Status::invalid_argument(format!(
            "{key} attribute must be true or false"
        ))),
    }
}

fn optional_role(
    attributes: &std::collections::HashMap<String, String>,
) -> Result<Option<WorkspaceRole>, Status> {
    match attributes.get("role").map(String::as_str) {
        None | Some("") => Ok(None),
        Some(role) => parse_role(role)
            .map(Some)
            .ok_or_else(|| Status::invalid_argument("role attribute is not supported")),
    }
}

fn optional_target_role(
    attributes: &std::collections::HashMap<String, String>,
) -> Result<Option<WorkspaceRole>, Status> {
    match attributes.get("target_role").map(String::as_str) {
        None | Some("") => Ok(None),
        Some(role) => parse_role(role)
            .map(Some)
            .ok_or_else(|| Status::invalid_argument("target_role attribute is not supported")),
    }
}

fn role_label(role: WorkspaceRole) -> String {
    role.to_string()
}
