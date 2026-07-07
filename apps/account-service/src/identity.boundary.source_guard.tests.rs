use std::{fs, path::Path};

const BOUNDED_TABLES: &[&str] = &[
    "workspaces",
    "workspace_memberships",
    "workspace_policies",
    "workspace_invitations",
    "tenant_policies",
    "tenant_break_glass_accounts",
    "access_review_campaigns",
    "access_review_items",
    "access_review_schedules",
    "tenant_domains",
    "federated_identity_providers",
    "scim_provisioning_connectors",
    "developer_health_checks",
    "developer_role_assignments",
    "developer_marketplace_apps",
    "developer_consent_screens",
    "developer_client_secret_versions",
    "developer_secret_rotations",
    "developer_sandbox_tenants",
    "developer_token_debug_sessions",
];

const ALLOWED_RUNTIME_MUTATION_FILES: &[&str] = &[
    "identity.domains.cloud.workspace_port.rs",
    "identity.domains.cloud.workspace_projection.rs",
    "identity.domains.enterprise.db.access_writes.rs",
    "identity.domains.enterprise.db.break_glass.rs",
    "identity.domains.enterprise.db.writes.rs",
    "identity.domains.federation.projection.domains.rs",
    "identity.domains.federation.projection.providers.rs",
    "identity.domains.federation.projection.scim.rs",
];

const OAUTH_TOKEN_REGION_FILES: &[&str] = &[
    "identity.domains.oauth.flows.tokens.generate.rs",
    "identity.domains.oauth.flows.refresh.rs",
    "identity.domains.oauth.flows.codes.rs",
    "identity.domains.oauth.device.exchange.rs",
];

const OAUTH_CLOUD_CONTEXT_FILES: &[&str] = &[
    "identity.domains.oauth.assurance.rs",
    "identity.domains.oauth.device.validation.rs",
    "identity.domains.oauth.clients.create.rs",
    "identity.domains.oauth.clients.list.rs",
    "identity.domains.oauth.flows.client_credentials.rs",
    "identity.domains.oauth.flows.tokens.introspect.rs",
    "identity.domains.oauth.flows.tokens.introspect.actor.rs",
];

const DEVELOPER_CLOUD_CONTEXT_FILES: &[&str] = &[
    "identity.domains.developer.apps.service.rs",
    "identity.domains.developer.service_accounts.db.rs",
];

const SECURITY_CLOUD_CONTEXT_FILES: &[&str] = &[
    "identity.domains.security.service.rs",
    "identity.domains.security.service.risk_events.rs",
];

const AUTH_CLOUD_CONTEXT_FILES: &[&str] = &[
    "identity.domains.auth.account_deletion.rs",
    "identity.domains.auth.audit.rs",
    "identity.domains.auth.mfa.policy.rs",
    "identity.domains.auth.password.db.rs",
    "identity.domains.auth.sessions.context.rs",
    "identity.domains.auth.sessions.context.workspace_switch.rs",
    "identity.domains.authz.db.rs",
];

const MEMBERS_CLOUD_CONTEXT_FILES: &[&str] = &[
    "identity.domains.members.membership.rs",
    "identity.domains.members.invites.logic.rs",
];

const WORKSPACES_CLOUD_CONTEXT_FILES: &[&str] = &[
    "identity.domains.workspaces.core.rs",
    "identity.domains.workspaces.db.rs",
    "identity.domains.workspaces.service.rs",
    "identity.domains.workspaces.settings.rs",
];

const ENTERPRISE_CLOUD_CONTEXT_FILES: &[&str] = &[
    "identity.domains.enterprise.db.access_reads.rs",
    "identity.domains.enterprise.db.reads.rs",
    "identity.domains.enterprise.db.reads_tenant.rs",
    "identity.domains.enterprise.db.scope_reads.rs",
    "identity.domains.enterprise.policy_simulation.db.rs",
    "identity.domains.enterprise.service.break_glass.rs",
    "identity.domains.enterprise.service.mutations.rs",
    "identity.domains.enterprise.service.reads.rs",
    "identity.domains.enterprise.service.access.rs",
    "identity.domains.enterprise.service.user_mutations.rs",
];

const FEDERATION_CLOUD_CONTEXT_FILES: &[&str] =
    &["identity.domains.federation.provisioning.principal.rs"];

#[test]
fn account_runtime_mutations_to_external_context_tables_go_through_boundary_ports() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();

    for entry in fs::read_dir(&src_dir).expect("account-service src should be readable") {
        let path = entry
            .expect("account-service src entry should be readable")
            .path();
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .expect("source file name should be valid UTF-8");
        if file_name.contains(".tests.") || ALLOWED_RUNTIME_MUTATION_FILES.contains(&file_name) {
            continue;
        }

        let source = fs::read_to_string(&path).expect("source file should be readable");
        let normalized = normalize_sql_source(&source);

        for table in BOUNDED_TABLES {
            if mutates_table(&normalized, table) {
                violations.push(format!("{file_name}: direct mutation of {table}"));
            }
        }

        if mutates_billing_table(&normalized) {
            violations.push(format!("{file_name}: direct mutation of billing_* table"));
        }
    }

    assert!(
        violations.is_empty(),
        "Account runtime code must mutate Cloud, Enterprise, and Billing-owned tables only through boundary ports.\nKnown Enterprise/federation compatibility paths are allowlisted in this test.\nViolations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn oauth_token_workspace_region_reads_go_through_cloud_boundary() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();

    for file_name in OAUTH_TOKEN_REGION_FILES {
        let path = src_dir.join(file_name);
        let source = fs::read_to_string(&path).expect("source file should be readable");
        let normalized = normalize_sql_source(&source);
        if normalized.contains("data_region")
            && (normalized.contains("from workspaces") || normalized.contains("join workspaces"))
        {
            violations.push((*file_name).to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "OAuth token issuance must read workspace region through the Cloud boundary, not Account SQL.\nViolations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn oauth_client_credentials_workspace_context_reads_go_through_cloud_boundary() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();

    for file_name in OAUTH_CLOUD_CONTEXT_FILES {
        let path = src_dir.join(file_name);
        let source = fs::read_to_string(&path).expect("source file should be readable");
        let normalized = normalize_sql_source(&source);
        if normalized.contains("from workspaces")
            || normalized.contains("join workspaces")
            || normalized.contains("from workspace_memberships")
            || normalized.contains("join workspace_memberships")
        {
            violations.push((*file_name).to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "OAuth workspace context helpers must read workspace context through the Cloud boundary, not Account SQL.\nViolations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn developer_workspace_context_reads_go_through_cloud_boundary() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();

    for file_name in DEVELOPER_CLOUD_CONTEXT_FILES {
        let path = src_dir.join(file_name);
        let source = fs::read_to_string(&path).expect("source file should be readable");
        let normalized = normalize_sql_source(&source);
        if normalized.contains("from workspaces")
            || normalized.contains("join workspaces")
            || normalized.contains("from workspace_memberships")
            || normalized.contains("join workspace_memberships")
        {
            violations.push((*file_name).to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "Developer runtime views must read workspace context through the Cloud boundary, not Account SQL.\nViolations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn security_workspace_context_reads_go_through_authorized_context() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();

    for file_name in SECURITY_CLOUD_CONTEXT_FILES {
        let path = src_dir.join(file_name);
        let source = fs::read_to_string(&path).expect("source file should be readable");
        let normalized = normalize_sql_source(&source);
        if normalized.contains("from workspaces")
            || normalized.contains("join workspaces")
            || normalized.contains("from workspace_memberships")
            || normalized.contains("join workspace_memberships")
        {
            violations.push((*file_name).to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "Security runtime views must use authorized workspace context instead of direct Cloud workspace SQL.\nViolations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn auth_workspace_context_reads_go_through_cloud_boundary() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();

    for file_name in AUTH_CLOUD_CONTEXT_FILES {
        let path = src_dir.join(file_name);
        let source = fs::read_to_string(&path).expect("source file should be readable");
        let normalized = normalize_sql_source(&source);
        if normalized.contains("from workspaces")
            || normalized.contains("join workspaces")
            || normalized.contains("from workspace_memberships")
            || normalized.contains("join workspace_memberships")
        {
            violations.push((*file_name).to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "Auth runtime helpers must read workspace context through the Cloud boundary, not Account SQL.\nViolations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn member_runtime_membership_reads_go_through_cloud_boundary() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();

    for file_name in MEMBERS_CLOUD_CONTEXT_FILES {
        let path = src_dir.join(file_name);
        let source = fs::read_to_string(&path).expect("source file should be readable");
        let normalized = normalize_sql_source(&source);
        if normalized.contains("from workspaces")
            || normalized.contains("join workspaces")
            || normalized.contains("from workspace_memberships")
            || normalized.contains("join workspace_memberships")
            || normalized.contains("from workspace_policies")
            || normalized.contains("join workspace_policies")
            || normalized.contains("from workspace_invitations")
            || normalized.contains("join workspace_invitations")
        {
            violations.push((*file_name).to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "Member runtime membership reads must go through the Cloud boundary, not Account SQL.\nViolations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn workspace_runtime_reads_go_through_cloud_boundary() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();

    for file_name in WORKSPACES_CLOUD_CONTEXT_FILES {
        let path = src_dir.join(file_name);
        let source = fs::read_to_string(&path).expect("source file should be readable");
        let normalized = normalize_sql_source(&source);
        if normalized.contains("from workspaces")
            || normalized.contains("join workspaces")
            || normalized.contains("from workspace_memberships")
            || normalized.contains("join workspace_memberships")
            || normalized.contains("from workspace_policies")
            || normalized.contains("join workspace_policies")
        {
            violations.push((*file_name).to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "Workspace runtime reads must go through the Cloud boundary, not Account SQL.\nViolations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn enterprise_runtime_reads_go_through_cloud_boundary() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();

    for file_name in ENTERPRISE_CLOUD_CONTEXT_FILES {
        let path = src_dir.join(file_name);
        let source = fs::read_to_string(&path).expect("source file should be readable");
        let normalized = normalize_sql_source(&source);
        if normalized.contains("from workspaces")
            || normalized.contains("join workspaces")
            || normalized.contains("from workspace_memberships")
            || normalized.contains("join workspace_memberships")
            || normalized.contains("from workspace_policies")
            || normalized.contains("join workspace_policies")
            || normalized.contains("from workspace_invitations")
            || normalized.contains("join workspace_invitations")
        {
            violations.push((*file_name).to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "Enterprise runtime reads must go through the Cloud boundary, not Account SQL.\nViolations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn federation_runtime_reads_go_through_cloud_boundary() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();

    for file_name in FEDERATION_CLOUD_CONTEXT_FILES {
        let path = src_dir.join(file_name);
        let source = fs::read_to_string(&path).expect("source file should be readable");
        let normalized = normalize_sql_source(&source);
        if normalized.contains("from workspaces")
            || normalized.contains("join workspaces")
            || normalized.contains("from workspace_memberships")
            || normalized.contains("join workspace_memberships")
            || normalized.contains("from workspace_policies")
            || normalized.contains("join workspace_policies")
        {
            violations.push((*file_name).to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "Federation runtime reads must go through the Cloud boundary, not Account SQL.\nViolations:\n{}",
        violations.join("\n")
    );
}

fn normalize_sql_source(source: &str) -> String {
    source
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn mutates_table(source: &str, table: &str) -> bool {
    ["insert into", "update", "delete from"]
        .into_iter()
        .any(|verb| source.contains(&format!("{verb} {table}")))
}

fn mutates_billing_table(source: &str) -> bool {
    [
        "insert into billing_",
        "update billing_",
        "delete from billing_",
    ]
    .into_iter()
    .any(|pattern| source.contains(pattern))
}
