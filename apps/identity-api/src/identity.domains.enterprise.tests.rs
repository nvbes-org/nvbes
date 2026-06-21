use crate::domains::enterprise::service::{
    create_invitations, get_billing, get_security, grant_admin_elevation, list_audit_events,
    list_users, list_workspaces, suspend_user, update_mfa_policy, update_session_policy,
    update_user_access,
};
use crate::domains::enterprise::types::{
    EnterpriseAccessUpdateInput, EnterpriseAdminElevationInput, EnterpriseInvitationInput,
    EnterpriseMfaPolicyInput, EnterpriseRole, EnterpriseSessionPolicyInput, EnterpriseSuspendInput,
};
use uuid::Uuid;

#[path = "identity.domains.enterprise.tests.support.rs"]
mod support;

#[tokio::test]
async fn test_delegated_administration_scoping_and_blocking() {
    let pool = crate::test_support::shared_test_pool();
    crate::test_support::ensure_test_database(&pool).await;
    let redis = crate::test_support::test_redis_pool().await;

    let (
        tenant_id,
        _org_a_id,
        _org_b_id,
        workspace_a_id,
        workspace_b_id,
        user_a_id,
        user_b_id,
        org_a_admin_auth,
    ) = support::setup_delegated_admin_test_data(&pool).await;

    let users_resp = list_users(&pool, &org_a_admin_auth, tenant_id)
        .await
        .unwrap();
    assert!(
        users_resp
            .users
            .iter()
            .any(|u| u.id == org_a_admin_auth.user_id)
    );
    assert!(users_resp.users.iter().any(|u| u.id == user_a_id));
    assert!(!users_resp.users.iter().any(|u| u.id == user_b_id));

    let workspaces_resp = list_workspaces(&pool, &org_a_admin_auth, tenant_id)
        .await
        .unwrap();
    assert!(
        workspaces_resp
            .workspaces
            .iter()
            .any(|w| w.id == workspace_a_id)
    );
    assert!(
        !workspaces_resp
            .workspaces
            .iter()
            .any(|w| w.id == workspace_b_id)
    );

    let audit_resp = list_audit_events(&pool, &org_a_admin_auth, tenant_id)
        .await
        .unwrap();
    for event in audit_resp.events {
        if let Some(ws_id) = event
            .metadata
            .as_ref()
            .and_then(|m| m.get("workspace_id"))
            .and_then(|v| v.as_str())
        {
            assert_eq!(ws_id, workspace_a_id.to_string());
        }
    }

    let billing_err = get_billing(&pool, &org_a_admin_auth, tenant_id)
        .await
        .unwrap_err();
    assert_eq!(billing_err.code, "tenant_scope_required");

    let security_err = get_security(&pool, &org_a_admin_auth, tenant_id, 12)
        .await
        .unwrap_err();
    assert_eq!(security_err.code, "tenant_scope_required");

    let elevation_err = grant_admin_elevation(
        &pool,
        &redis,
        &org_a_admin_auth,
        tenant_id,
        EnterpriseAdminElevationInput {
            duration_minutes: Some(15),
            reason: Some("scope check".to_string()),
            procedure_reference: None,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(elevation_err.code, "tenant_scope_required");

    let session_policy_err = update_session_policy(
        &pool,
        &redis,
        &org_a_admin_auth,
        tenant_id,
        12,
        EnterpriseSessionPolicyInput {
            admin_session_ttl_hours: 8,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(session_policy_err.code, "tenant_scope_required");

    let mfa_policy_err = update_mfa_policy(
        &pool,
        &redis,
        &org_a_admin_auth,
        tenant_id,
        12,
        EnterpriseMfaPolicyInput {
            policy: "required_admins".to_string(),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(mfa_policy_err.code, "tenant_scope_required");

    let trust_err =
        crate::domains::enterprise::trust::get_trust_center(&pool, &org_a_admin_auth, tenant_id)
            .await
            .unwrap_err();
    assert_eq!(trust_err.code, "tenant_scope_required");

    let access_reviews_err = crate::domains::enterprise::access_reviews::service::list_campaigns(
        &pool,
        &org_a_admin_auth,
        tenant_id,
    )
    .await
    .unwrap_err();
    assert_eq!(access_reviews_err.code, "tenant_scope_required");

    let policy_simulation_err = crate::domains::enterprise::policy_simulation::simulate_policy(
        &pool,
        &org_a_admin_auth,
        tenant_id,
        crate::domains::enterprise::policy_simulation::EnterprisePolicySimulationInput {
            workspace_id: workspace_a_id,
            subject:
                crate::domains::enterprise::policy_simulation::types::EnterprisePolicySimulationSubject::User {
                    user_id: user_a_id,
                },
            action: "workspace.member.list".to_string(),
            resource: None,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(policy_simulation_err.code, "tenant_scope_required");

    let invite_ok = create_invitations(
        &pool,
        &redis,
        &org_a_admin_auth,
        tenant_id,
        EnterpriseInvitationInput {
            emails: vec!["new_member@example.test".to_string()],
            role: EnterpriseRole::Member,
            module_grants: vec![],
            workspace_ids: vec![workspace_a_id],
        },
    )
    .await;
    assert!(invite_ok.is_ok());

    let invite_err = create_invitations(
        &pool,
        &redis,
        &org_a_admin_auth,
        tenant_id,
        EnterpriseInvitationInput {
            emails: vec!["new_member_blocked@example.test".to_string()],
            role: EnterpriseRole::Member,
            module_grants: vec![],
            workspace_ids: vec![workspace_b_id],
        },
    )
    .await
    .unwrap_err();
    assert_eq!(invite_err.code, "invalid_workspace_scope");

    let update_ok = update_user_access(
        &pool,
        &redis,
        &org_a_admin_auth,
        tenant_id,
        user_a_id,
        EnterpriseAccessUpdateInput {
            role: EnterpriseRole::Member,
            module_grants: vec![],
            workspace_ids: vec![workspace_a_id],
        },
    )
    .await;
    assert!(update_ok.is_ok());

    let user_a_workspace_b_status = sqlx::query_scalar::<_, String>(
        "SELECT status::text FROM workspace_memberships WHERE workspace_id = $1 AND principal_id = $2",
    )
    .bind(workspace_b_id)
    .bind(user_a_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(user_a_workspace_b_status, "active");

    let suspend_ok = suspend_user(
        &pool,
        &redis,
        &org_a_admin_auth,
        tenant_id,
        user_a_id,
        EnterpriseSuspendInput {
            reason: "org_a_offboarding".to_string(),
        },
    )
    .await;
    assert!(suspend_ok.is_ok());

    let user_a_workspace_statuses = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT workspace_id, status::text FROM workspace_memberships WHERE principal_id = $1 AND workspace_id IN ($2, $3)",
    )
    .bind(user_a_id)
    .bind(workspace_a_id)
    .bind(workspace_b_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(
        user_a_workspace_statuses
            .iter()
            .any(|(workspace_id, status)| *workspace_id == workspace_a_id && status == "suspended")
    );
    assert!(
        user_a_workspace_statuses
            .iter()
            .any(|(workspace_id, status)| *workspace_id == workspace_b_id && status == "active")
    );

    let scoped_audit_after_mutations = list_audit_events(&pool, &org_a_admin_auth, tenant_id)
        .await
        .unwrap();
    assert!(
        scoped_audit_after_mutations
            .events
            .iter()
            .any(
                |event| event.event_type == "enterprise.member.access_updated"
                    && event.target_id == Some(user_a_id)
            )
    );
    assert!(
        scoped_audit_after_mutations
            .events
            .iter()
            .any(|event| event.event_type == "enterprise.member.suspended"
                && event.target_id == Some(user_a_id))
    );

    let update_err = update_user_access(
        &pool,
        &redis,
        &org_a_admin_auth,
        tenant_id,
        user_b_id,
        EnterpriseAccessUpdateInput {
            role: EnterpriseRole::Member,
            module_grants: vec![],
            workspace_ids: vec![workspace_a_id],
        },
    )
    .await
    .unwrap_err();
    assert_eq!(update_err.code, "user_not_in_organization");
}
