use sqlx::Row;
use std::collections::HashMap;

use crate::{
    grpc::{
        admin_elevation, break_glass, federation, pb::nvbes::enterprise::v1 as enterprise,
        policies, policy_evaluation,
    },
    test_support::{
        cleanup_tenant, has_enterprise_contract_schema, seed_enterprise_fixture, test_pool,
    },
};

#[tokio::test]
async fn policy_updates_round_trip_through_enterprise_store() {
    let pool = test_pool();
    if !has_enterprise_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Enterprise contract schema");
        return;
    }

    let fixture = seed_enterprise_fixture(&pool, "policy").await;
    let session = policies::update_policy(
        &pool,
        fixture.tenant_id,
        "session",
        r#"{"admin_session_ttl_hours":6}"#,
    )
    .await
    .expect("session policy should be updated");
    let mfa = policies::update_policy(
        &pool,
        fixture.tenant_id,
        "mfa",
        r#"{"policy":"required_admins"}"#,
    )
    .await
    .expect("mfa policy should be updated");

    assert_eq!(session.policy_kind, "session");
    assert_eq!(mfa.policy_kind, "mfa");

    let policy_set = policies::policy_set(&pool, fixture.tenant_id)
        .await
        .expect("policy set should be readable");
    assert_eq!(policy_set.tenant_id, fixture.tenant_id.to_string());
    assert!(policy_set.policies.iter().any(|policy| {
        policy.policy_kind == "session" && policy.rules_json.contains("\"admin_session_ttl_hours\"")
    }));
    assert!(policy_set.policies.iter().any(|policy| {
        policy.policy_kind == "mfa" && policy.rules_json.contains("required_admins")
    }));

    cleanup_tenant(&pool, fixture.tenant_id).await;
}

#[tokio::test]
async fn trust_center_derives_security_posture_from_enterprise_tables() {
    let pool = test_pool();
    if !has_enterprise_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Enterprise contract schema");
        return;
    }

    let fixture = seed_enterprise_fixture(&pool, "trust-center").await;
    let provider = federation::configure_federation_provider(
        &pool,
        fixture.tenant_id,
        enterprise::ConfigureFederationProviderRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            provider_id: String::new(),
            protocol: "saml".to_string(),
            issuer: "https://idp.trust.example.test".to_string(),
            metadata_url: "https://idp.trust.example.test/metadata".to_string(),
            scim_base_url: String::new(),
            status: "active".to_string(),
            provider_family: "okta".to_string(),
            name: "Okta trust".to_string(),
            client_id: String::new(),
            sp_entity_id: "urn:nvbes:trust".to_string(),
            attribute_mapping_json: r#"{"email":"email"}"#.to_string(),
            encryption_cert_pem: String::new(),
            require_signed_assertions: true,
            require_signed_responses: true,
        },
    )
    .await
    .expect("federation provider should be configured");
    let domain = federation::configure_tenant_domain(
        &pool,
        fixture.tenant_id,
        enterprise::ConfigureTenantDomainRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            domain_id: String::new(),
            domain: "trust.example.test".to_string(),
            sso_required: true,
            sso_provider_id: provider.provider_id.clone(),
            verification_token_hash: sha256("trust-token"),
        },
    )
    .await
    .expect("tenant domain should be configured");
    let domain_id = uuid::Uuid::parse_str(&domain.domain_id).expect("domain id should parse");
    let domain =
        federation::verify_tenant_domain(&pool, fixture.tenant_id, domain_id, "trust-token")
            .await
            .expect("tenant domain should be verified");
    seed_trust_center_posture(&pool, &fixture).await;

    let trust_center = crate::grpc::trust::trust_center(&pool, fixture.tenant_id)
        .await
        .expect("trust center should be readable");
    let tenant = trust_center.tenant.expect("tenant should be present");
    let mfa = trust_center.mfa.expect("mfa should be present");
    let sso = trust_center.sso.expect("sso should be present");
    let audit = trust_center.audit.expect("audit should be present");
    let dpa = trust_center.dpa.expect("dpa should be present");

    assert_eq!(tenant.tenant_id, fixture.tenant_id.to_string());
    assert!(mfa.enabled);
    assert_eq!(mfa.active_members, 2);
    assert_eq!(mfa.members_with_mfa, 1);
    assert_eq!(mfa.passkey_factors, 1);
    assert!(sso.enabled);
    assert_eq!(sso.active_providers, 1);
    assert_eq!(sso.required_domains, 1);
    assert_eq!(sso.providers[0].provider_id, provider.provider_id);
    assert_eq!(trust_center.verified_domains[0].domain_id, domain.domain_id);
    assert!(trust_center.verified_domains[0].verified);
    assert!(audit.immutable);
    assert_eq!(audit.recent_events[0].event_type, "trust.center.viewed");
    assert_eq!(trust_center.hosting_regions[0].data_region, "eu");
    assert_eq!(trust_center.hosting_regions[0].legal_jurisdiction, "gdpr");
    assert_eq!(trust_center.hosting_regions[0].workspace_count, 1);
    assert_eq!(dpa.version, "2026-05-11");
    assert!(trust_center.subprocessors.is_empty());

    cleanup_tenant(&pool, fixture.tenant_id).await;
}

fn sha256(value: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

#[test]
fn admin_elevation_authorization_bounds_expiry_and_requires_admin_role() {
    let now = chrono::Utc::now();
    let tenant_id = uuid::Uuid::new_v4();
    let authorized = admin_elevation::authorize_admin_elevation_decision(
        tenant_id,
        &enterprise::AuthorizeAdminElevationRequest {
            context: None,
            tenant_id: tenant_id.to_string(),
            base_role: "owner".to_string(),
            duration_minutes: 60,
            step_up_expires_at: (now + chrono::Duration::minutes(12)).to_rfc3339(),
            session_expires_at: (now + chrono::Duration::minutes(30)).to_rfc3339(),
            break_glass: false,
            break_glass_reason: String::new(),
            break_glass_procedure_reference: String::new(),
            reason: "time-bound administration".to_string(),
            authentication: Some(privileged_authentication_context(now.timestamp())),
        },
    )
    .expect("owner should be authorized for admin elevation");

    let expires_at =
        chrono::DateTime::parse_from_rfc3339(&authorized.expires_at).expect("expiry should parse");
    assert_eq!(authorized.tenant_id, tenant_id.to_string());
    assert_eq!(authorized.role, "admin");
    assert!(expires_at <= now + chrono::Duration::minutes(12));

    let denied = admin_elevation::authorize_admin_elevation_decision(
        tenant_id,
        &enterprise::AuthorizeAdminElevationRequest {
            context: None,
            tenant_id: tenant_id.to_string(),
            base_role: "member".to_string(),
            duration_minutes: 15,
            step_up_expires_at: (now + chrono::Duration::minutes(12)).to_rfc3339(),
            session_expires_at: (now + chrono::Duration::minutes(30)).to_rfc3339(),
            break_glass: false,
            break_glass_reason: String::new(),
            break_glass_procedure_reference: String::new(),
            reason: "time-bound administration".to_string(),
            authentication: Some(privileged_authentication_context(now.timestamp())),
        },
    )
    .expect_err("member should not be authorized for admin elevation");
    assert_eq!(denied.code(), tonic::Code::PermissionDenied);
}

fn privileged_authentication_context(
    auth_time: i64,
) -> enterprise::PrivilegedAuthenticationContext {
    enterprise::PrivilegedAuthenticationContext {
        acr: "aal2".to_string(),
        amr: vec!["webauthn".to_string()],
        auth_time,
        authentication_event_id: "contract-authn-event".to_string(),
    }
}

#[test]
fn policy_simulation_allows_owner_and_reports_step_up() {
    let decision = policy_evaluation::evaluate_with_context(
        enterprise::EvaluatePolicyRequest {
            context: None,
            tenant_id: uuid::Uuid::new_v4().to_string(),
            subject_principal_id: uuid::Uuid::new_v4().to_string(),
            action: "delete_workspace".to_string(),
            resource: "workspace".to_string(),
            attributes: policy_attributes([
                ("workspace_id", uuid::Uuid::new_v4().to_string()),
                ("subject_type", "user".to_string()),
                ("subject_id", uuid::Uuid::new_v4().to_string()),
                ("subject_label", "Owner".to_string()),
                ("email_verified", "true".to_string()),
                ("role", "owner".to_string()),
                ("owns_resource", "false".to_string()),
                ("member_share_links_enabled", "false".to_string()),
                ("target_role", String::new()),
            ]),
        },
        &Default::default(),
        None,
        true,
    )
    .expect("policy simulation should evaluate");

    assert_eq!(decision.result, "allow");
    assert_eq!(decision.reason, "allowed");
    assert_eq!(decision.role, "owner");
    assert!(decision.requires_step_up);
}

#[test]
fn policy_simulation_denies_unverified_user_before_role_policy() {
    let decision = policy_evaluation::evaluate_with_context(
        enterprise::EvaluatePolicyRequest {
            context: None,
            tenant_id: uuid::Uuid::new_v4().to_string(),
            subject_principal_id: uuid::Uuid::new_v4().to_string(),
            action: "view_files".to_string(),
            resource: "workspace".to_string(),
            attributes: policy_attributes([
                ("workspace_id", uuid::Uuid::new_v4().to_string()),
                ("subject_type", "user".to_string()),
                ("subject_id", uuid::Uuid::new_v4().to_string()),
                ("subject_label", "Member".to_string()),
                ("email_verified", "false".to_string()),
                ("role", "owner".to_string()),
                ("owns_resource", "false".to_string()),
                ("member_share_links_enabled", "false".to_string()),
                ("target_role", String::new()),
            ]),
        },
        &Default::default(),
        None,
        false,
    )
    .expect("policy simulation should evaluate");

    assert_eq!(decision.result, "deny");
    assert_eq!(decision.reason, "email_not_verified");
    assert!(!decision.requires_step_up);
}

#[tokio::test]
async fn federation_governance_round_trip_includes_provider_domain_and_scim() {
    let pool = test_pool();
    if !has_enterprise_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Enterprise contract schema");
        return;
    }

    let fixture = seed_enterprise_fixture(&pool, "federation-governance").await;
    let provider = federation::configure_federation_provider(
        &pool,
        fixture.tenant_id,
        enterprise::ConfigureFederationProviderRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            provider_id: String::new(),
            protocol: "oidc".to_string(),
            issuer: "https://idp.example.test".to_string(),
            metadata_url: "https://idp.example.test/.well-known/openid-configuration".to_string(),
            scim_base_url: String::new(),
            status: "active".to_string(),
            provider_family: "okta".to_string(),
            name: "Okta workforce".to_string(),
            client_id: "client-id".to_string(),
            sp_entity_id: String::new(),
            attribute_mapping_json: r#"{"email":"email"}"#.to_string(),
            encryption_cert_pem: String::new(),
            require_signed_assertions: true,
            require_signed_responses: true,
        },
    )
    .await
    .expect("federation provider should be configured");
    let domain = federation::configure_tenant_domain(
        &pool,
        fixture.tenant_id,
        enterprise::ConfigureTenantDomainRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            domain_id: String::new(),
            domain: "example.test".to_string(),
            sso_required: true,
            sso_provider_id: provider.provider_id.clone(),
            verification_token_hash: "hashed-token".to_string(),
        },
    )
    .await
    .expect("tenant domain should be configured");
    let connector = federation::configure_scim_connector(
        &pool,
        fixture.tenant_id,
        enterprise::ConfigureScimConnectorRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            connector_id: String::new(),
            provider: "okta".to_string(),
            status: "active".to_string(),
            base_url: "https://idp.example.test/scim/v2".to_string(),
            credential_ttl_hours: 24,
        },
    )
    .await
    .expect("SCIM connector should be configured");

    let governance = federation::federation_governance(&pool, fixture.tenant_id)
        .await
        .expect("federation governance should be readable");

    assert_eq!(governance.tenant_id, fixture.tenant_id.to_string());
    assert_eq!(governance.providers.len(), 1);
    assert_eq!(governance.providers[0].provider_id, provider.provider_id);
    assert_eq!(governance.providers[0].provider_family, "okta");
    assert_eq!(governance.domains.len(), 1);
    assert_eq!(governance.domains[0].domain_id, domain.domain_id);
    assert!(governance.domains[0].sso_required);
    assert_eq!(governance.scim_connectors.len(), 1);
    assert_eq!(
        governance.scim_connectors[0].connector_id,
        connector.connector_id
    );
    assert!(governance.scim_required);

    cleanup_tenant(&pool, fixture.tenant_id).await;
}

fn policy_attributes(
    entries: impl IntoIterator<Item = (&'static str, String)>,
) -> HashMap<String, String> {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect()
}

async fn seed_trust_center_posture(
    pool: &sqlx::PgPool,
    fixture: &crate::test_support::EnterpriseFixture,
) {
    let workspace_id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO workspaces (
          id, tenant_id, name, workspace_type, owner_user_id, plan_code,
          data_region, jurisdiction, created_at, updated_at
        )
        VALUES ($1, $2, 'Trust workspace', 'team', $3, 'team', 'eu', 'gdpr', $4, $4)
        "#,
    )
    .bind(workspace_id)
    .bind(fixture.tenant_id)
    .bind(fixture.actor_id)
    .bind(fixture.now)
    .execute(pool)
    .await
    .expect("workspace should be seeded");

    sqlx::query(
        r#"
        INSERT INTO mfa_factors (
          principal_id, factor_type, status, label, confirmed_at, created_at
        )
        VALUES ($1, 'webauthn', 'active', 'Security key', $2, $2)
        "#,
    )
    .bind(fixture.principal_id)
    .bind(fixture.now)
    .execute(pool)
    .await
    .expect("mfa factor should be seeded");

    sqlx::query(
        r#"
        INSERT INTO audit_events (
          tenant_id, actor_principal_id, action, target_type, target_id,
          metadata, event_hash, created_at
        )
        VALUES ($1, $2, 'trust.center.viewed', 'tenant', $1, $3, $4, $5)
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(fixture.actor_id)
    .bind(serde_json::json!({ "source": "enterprise-contract" }))
    .bind(format!("trust-center-{}", fixture.tenant_id))
    .bind(fixture.now)
    .execute(pool)
    .await
    .expect("audit event should be seeded");
}

#[path = "enterprise.grpc.service.contract_tests.break_glass.rs"]
mod break_glass_tests;
