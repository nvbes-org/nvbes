use super::types::{EnterpriseModuleGrant, EnterpriseRole};

pub fn can_manage_members(role: &str, grants: &[String]) -> bool {
    role == "owner" || (role == "admin" && grants.iter().any(|grant| grant == "members"))
}

pub fn is_last_owner_removal(owner_count: i64, current_role: &str, next_role: &str) -> bool {
    owner_count <= 1 && current_role == "owner" && next_role != "owner"
}

pub fn role_from_db(value: &str) -> EnterpriseRole {
    match value {
        "owner" => EnterpriseRole::Owner,
        "admin" => EnterpriseRole::Admin,
        "member" => EnterpriseRole::Member,
        _ => EnterpriseRole::Viewer,
    }
}

pub fn role_as_db(role: &EnterpriseRole) -> &'static str {
    match role {
        EnterpriseRole::Owner => "owner",
        EnterpriseRole::Admin => "admin",
        EnterpriseRole::Member => "member",
        EnterpriseRole::Viewer => "viewer",
    }
}

pub fn grants_for_role(role: &EnterpriseRole) -> Vec<EnterpriseModuleGrant> {
    match role {
        EnterpriseRole::Owner => vec![
            EnterpriseModuleGrant::Members,
            EnterpriseModuleGrant::Workspaces,
            EnterpriseModuleGrant::Developers,
            EnterpriseModuleGrant::Policies,
            EnterpriseModuleGrant::Security,
            EnterpriseModuleGrant::Billing,
            EnterpriseModuleGrant::Audit,
            EnterpriseModuleGrant::Drive,
        ],
        EnterpriseRole::Admin => vec![
            EnterpriseModuleGrant::Members,
            EnterpriseModuleGrant::Workspaces,
            EnterpriseModuleGrant::Audit,
            EnterpriseModuleGrant::Drive,
        ],
        EnterpriseRole::Member => vec![EnterpriseModuleGrant::Drive],
        EnterpriseRole::Viewer => Vec::new(),
    }
}

pub fn grant_names(grants: &[EnterpriseModuleGrant]) -> Vec<String> {
    grants.iter().map(grant_as_db).map(str::to_string).collect()
}

fn grant_as_db(grant: &EnterpriseModuleGrant) -> &'static str {
    match grant {
        EnterpriseModuleGrant::Members => "members",
        EnterpriseModuleGrant::Workspaces => "workspaces",
        EnterpriseModuleGrant::Developers => "developers",
        EnterpriseModuleGrant::Policies => "policies",
        EnterpriseModuleGrant::Security => "security",
        EnterpriseModuleGrant::Billing => "billing",
        EnterpriseModuleGrant::Audit => "audit",
        EnterpriseModuleGrant::Drive => "drive",
    }
}
