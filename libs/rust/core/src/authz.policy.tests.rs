use super::*;
use crate::authz::action::{action_requires_independent_approval, action_requires_step_up};

#[test]
fn owner_can_manage_workspace_and_billing() {
    assert!(is_allowed(
        WorkspaceRole::Owner,
        WorkspaceAction::UpdateWorkspaceSettings,
        ResourceContext::default()
    ));
    assert!(is_allowed(
        WorkspaceRole::Owner,
        WorkspaceAction::ManageBilling,
        ResourceContext::default()
    ));
}

#[test]
fn admin_cannot_manage_billing_or_delete_workspace() {
    assert!(!is_allowed(
        WorkspaceRole::Admin,
        WorkspaceAction::ManageBilling,
        ResourceContext::default()
    ));
    assert!(!is_allowed(
        WorkspaceRole::Admin,
        WorkspaceAction::DeleteWorkspace,
        ResourceContext::default()
    ));
}

#[test]
fn admin_can_only_invite_member_or_viewer() {
    assert!(is_allowed(
        WorkspaceRole::Admin,
        WorkspaceAction::InviteMember,
        ResourceContext {
            target_role: Some(WorkspaceRole::Member),
            ..ResourceContext::default()
        }
    ));
    assert!(!is_allowed(
        WorkspaceRole::Admin,
        WorkspaceAction::InviteMember,
        ResourceContext {
            target_role: Some(WorkspaceRole::Admin),
            ..ResourceContext::default()
        }
    ));
}

#[test]
fn security_admin_can_view_audit_but_cannot_invite_members() {
    assert!(is_allowed(
        WorkspaceRole::SecurityAdmin,
        WorkspaceAction::ViewAudit,
        ResourceContext::default()
    ));
    assert!(!is_allowed(
        WorkspaceRole::SecurityAdmin,
        WorkspaceAction::InviteMember,
        ResourceContext {
            target_role: Some(WorkspaceRole::Member),
            ..ResourceContext::default()
        }
    ));
    assert!(is_allowed(
        WorkspaceRole::SecurityAdmin,
        WorkspaceAction::ManageKeys,
        ResourceContext::default()
    ));
}

#[test]
fn billing_admin_can_manage_billing_but_cannot_view_audit() {
    assert!(is_allowed(
        WorkspaceRole::BillingAdmin,
        WorkspaceAction::ManageBilling,
        ResourceContext::default()
    ));
    assert!(!is_allowed(
        WorkspaceRole::BillingAdmin,
        WorkspaceAction::ViewAudit,
        ResourceContext::default()
    ));
}

#[test]
fn workspace_membership_roles_cover_expected_permission_boundaries() {
    let cases = [
        (WorkspaceRole::Owner, WorkspaceAction::DeleteWorkspace, true),
        (WorkspaceRole::Admin, WorkspaceAction::UploadFile, true),
        (WorkspaceRole::Admin, WorkspaceAction::ManageBilling, false),
        (
            WorkspaceRole::SecurityAdmin,
            WorkspaceAction::ExportAudit,
            true,
        ),
        (
            WorkspaceRole::SecurityAdmin,
            WorkspaceAction::UploadFile,
            false,
        ),
        (
            WorkspaceRole::BillingAdmin,
            WorkspaceAction::ManageBilling,
            true,
        ),
        (
            WorkspaceRole::BillingAdmin,
            WorkspaceAction::ViewAudit,
            false,
        ),
        (WorkspaceRole::Member, WorkspaceAction::UploadFile, true),
        (WorkspaceRole::Member, WorkspaceAction::ViewAudit, false),
        (WorkspaceRole::Viewer, WorkspaceAction::DownloadFile, true),
        (WorkspaceRole::Viewer, WorkspaceAction::UploadFile, false),
    ];

    for (role, action, expected) in cases {
        assert_eq!(
            is_allowed(role, action, ResourceContext::default()),
            expected,
            "{role:?} {action:?}"
        );
    }
}

#[test]
fn member_can_only_modify_owned_objects() {
    assert!(is_allowed(
        WorkspaceRole::Member,
        WorkspaceAction::RenameObject,
        ResourceContext {
            owns_resource: true,
            ..ResourceContext::default()
        }
    ));
    assert!(!is_allowed(
        WorkspaceRole::Member,
        WorkspaceAction::RenameObject,
        ResourceContext::default()
    ));
}

#[test]
fn member_share_link_requires_policy_and_ownership() {
    assert!(is_allowed(
        WorkspaceRole::Member,
        WorkspaceAction::CreateShareLink,
        ResourceContext {
            owns_resource: true,
            member_share_links_enabled: true,
            ..ResourceContext::default()
        }
    ));
    assert!(!is_allowed(
        WorkspaceRole::Member,
        WorkspaceAction::CreateShareLink,
        ResourceContext {
            owns_resource: true,
            member_share_links_enabled: false,
            ..ResourceContext::default()
        }
    ));
}

#[test]
fn viewer_is_read_only() {
    assert!(is_allowed(
        WorkspaceRole::Viewer,
        WorkspaceAction::DownloadFile,
        ResourceContext::default()
    ));
    assert!(!is_allowed(
        WorkspaceRole::Viewer,
        WorkspaceAction::CreateFolder,
        ResourceContext::default()
    ));
}

#[test]
fn owner_cannot_invite_or_remove_another_owner() {
    assert!(!is_allowed(
        WorkspaceRole::Owner,
        WorkspaceAction::InviteMember,
        ResourceContext {
            target_role: Some(WorkspaceRole::Owner),
            ..ResourceContext::default()
        }
    ));
    assert!(!is_allowed(
        WorkspaceRole::Owner,
        WorkspaceAction::RemoveMember,
        ResourceContext {
            target_role: Some(WorkspaceRole::Owner),
            ..ResourceContext::default()
        }
    ));
}

#[test]
fn sensitive_actions_require_step_up() {
    assert!(action_requires_step_up(WorkspaceAction::ManageBilling));
    assert!(action_requires_step_up(
        WorkspaceAction::ExportWorkspaceData
    ));
    assert!(action_requires_step_up(WorkspaceAction::DeleteWorkspace));
    assert!(!action_requires_step_up(WorkspaceAction::ViewBilling));
    assert!(!action_requires_step_up(WorkspaceAction::ViewAudit));
}

#[test]
fn separation_of_duties_covers_privileged_domains() {
    for action in [
        WorkspaceAction::ManageBilling,
        WorkspaceAction::ManageKeys,
        WorkspaceAction::ManageAdministration,
        WorkspaceAction::ExportAudit,
        WorkspaceAction::ExportWorkspaceData,
        WorkspaceAction::DeleteWorkspace,
    ] {
        assert!(
            action_requires_independent_approval(action),
            "{action:?} must require an independent approver"
        );
    }
}
