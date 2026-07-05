use sha2::{Digest, Sha256};

pub(super) fn auth_scope(scopes: &[String]) -> String {
    scopes
        .iter()
        .map(|scope| public_scope_to_drive_scope(scope).unwrap_or(scope.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn scope_allows_required(scopes: &[String], required_scope: &str) -> bool {
    let mapped_scope = public_scope_to_drive_scope(required_scope);
    let legacy_mapped_scope = public_scope_to_legacy_drive_scope(required_scope);

    scopes.iter().any(|scope| {
        scope == required_scope
            || mapped_scope.is_some_and(|mapped| scope == mapped)
            || legacy_mapped_scope.is_some_and(|mapped| scope == mapped)
            || scope == "drive:admin"
            || scope == "drive.admin"
    })
}

fn public_scope_to_drive_scope(scope: &str) -> Option<&'static str> {
    match scope {
        "files:read" => Some("drive.files.read"),
        "files:write" => Some("drive.files.write"),
        "files:delete" => Some("drive.files.delete"),
        "workspaces:read" => Some("drive.workspace.read"),
        "workspaces:write" | "workspaces:manage" => Some("drive.workspace.manage"),
        "share_links:read" => Some("drive.share_links.read"),
        "share_links:write" => Some("drive.share_links.write"),
        "audit:read" | "audit_events:read" => Some("drive.audit.read"),
        "quota:read" | "quotas:read" => Some("drive.quota.read"),
        _ => None,
    }
}

fn public_scope_to_legacy_drive_scope(scope: &str) -> Option<&'static str> {
    match scope {
        "files:read" => Some("drive:file:read"),
        "files:write" => Some("drive:file:write"),
        "files:delete" => Some("drive:file:delete"),
        "workspaces:read" => Some("drive:workspace:read"),
        "workspaces:write" | "workspaces:manage" => Some("drive:workspace:manage"),
        "share_links:read" => Some("drive:share_link:read"),
        "share_links:write" => Some("drive:share_link:write"),
        "audit:read" | "audit_events:read" => Some("drive:audit_event:read"),
        "quota:read" | "quotas:read" => Some("drive:quota:read"),
        _ => None,
    }
}

pub(super) fn key_hash(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    format!("{digest:x}")
}
