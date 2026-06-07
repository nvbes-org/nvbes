use crate::{
    domains::{
        auth::types::AuthPrincipalKind,
        authz::types::{WorkspaceRole, parse_role},
    },
    http::error::AppError,
};

pub(super) fn narrow_role(left: WorkspaceRole, right: WorkspaceRole) -> WorkspaceRole {
    if role_rank(left) >= role_rank(right) {
        right
    } else {
        left
    }
}

fn role_rank(role: WorkspaceRole) -> u8 {
    match role {
        WorkspaceRole::Owner => 3,
        WorkspaceRole::Admin => 2,
        WorkspaceRole::Member => 1,
        WorkspaceRole::Viewer => 0,
    }
}

pub(super) fn parse_token_role(token_role: &str) -> Result<WorkspaceRole, AppError> {
    parse_role(token_role)
}

pub(super) fn principal_type_label(kind: AuthPrincipalKind) -> &'static str {
    match kind {
        AuthPrincipalKind::User => "user",
        AuthPrincipalKind::ServiceAccount => "service_account",
    }
}
