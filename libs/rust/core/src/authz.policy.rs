use super::{action::WorkspaceAction, context::ResourceContext, role::WorkspaceRole};

pub fn is_allowed(role: WorkspaceRole, action: WorkspaceAction, ctx: ResourceContext) -> bool {
    match role {
        WorkspaceRole::Owner => owner_allows(action, ctx),
        WorkspaceRole::Admin => admin_allows(action, ctx),
        WorkspaceRole::Member => member_allows(action, ctx),
        WorkspaceRole::Viewer => viewer_allows(action),
    }
}

fn owner_allows(action: WorkspaceAction, ctx: ResourceContext) -> bool {
    match action {
        WorkspaceAction::ViewWorkspace
        | WorkspaceAction::UpdateWorkspaceSettings
        | WorkspaceAction::ViewMembers
        | WorkspaceAction::InviteMember
        | WorkspaceAction::ChangeMemberRole
        | WorkspaceAction::RemoveMember
        | WorkspaceAction::ViewFiles
        | WorkspaceAction::ViewTrash
        | WorkspaceAction::CreateFolder
        | WorkspaceAction::UploadFile
        | WorkspaceAction::RenameObject
        | WorkspaceAction::MoveObject
        | WorkspaceAction::TrashObject
        | WorkspaceAction::RestoreObject
        | WorkspaceAction::DeleteObjectPermanently
        | WorkspaceAction::DownloadFile
        | WorkspaceAction::ViewShareLinks
        | WorkspaceAction::CreateShareLink
        | WorkspaceAction::UpdateShareLink
        | WorkspaceAction::RevokeShareLink
        | WorkspaceAction::ViewQuota
        | WorkspaceAction::ViewBilling
        | WorkspaceAction::ManageBilling
        | WorkspaceAction::ViewAudit
        | WorkspaceAction::ExportAudit
        | WorkspaceAction::ExportWorkspaceData
        | WorkspaceAction::DeleteWorkspace => {
            if matches!(
                action,
                WorkspaceAction::RemoveMember
                    | WorkspaceAction::InviteMember
                    | WorkspaceAction::ChangeMemberRole
            ) && matches!(ctx.target_role, Some(WorkspaceRole::Owner))
            {
                return false;
            }

            true
        }
    }
}

fn admin_allows(action: WorkspaceAction, ctx: ResourceContext) -> bool {
    match action {
        WorkspaceAction::ViewWorkspace
        | WorkspaceAction::ViewMembers
        | WorkspaceAction::ViewFiles
        | WorkspaceAction::ViewTrash
        | WorkspaceAction::CreateFolder
        | WorkspaceAction::UploadFile
        | WorkspaceAction::RenameObject
        | WorkspaceAction::MoveObject
        | WorkspaceAction::TrashObject
        | WorkspaceAction::RestoreObject
        | WorkspaceAction::DownloadFile
        | WorkspaceAction::ViewShareLinks
        | WorkspaceAction::CreateShareLink
        | WorkspaceAction::UpdateShareLink
        | WorkspaceAction::RevokeShareLink
        | WorkspaceAction::ViewQuota
        | WorkspaceAction::ViewAudit
        | WorkspaceAction::ExportAudit => true,
        WorkspaceAction::InviteMember | WorkspaceAction::RemoveMember => {
            matches!(
                ctx.target_role,
                Some(WorkspaceRole::Member | WorkspaceRole::Viewer)
            )
        }
        WorkspaceAction::ChangeMemberRole
        | WorkspaceAction::UpdateWorkspaceSettings
        | WorkspaceAction::DeleteObjectPermanently
        | WorkspaceAction::ViewBilling
        | WorkspaceAction::ManageBilling
        | WorkspaceAction::ExportWorkspaceData
        | WorkspaceAction::DeleteWorkspace => false,
    }
}

fn member_allows(action: WorkspaceAction, ctx: ResourceContext) -> bool {
    match action {
        WorkspaceAction::ViewWorkspace
        | WorkspaceAction::ViewFiles
        | WorkspaceAction::ViewTrash
        | WorkspaceAction::DownloadFile
        | WorkspaceAction::ViewQuota => true,
        WorkspaceAction::CreateFolder | WorkspaceAction::UploadFile => true,
        WorkspaceAction::RenameObject
        | WorkspaceAction::MoveObject
        | WorkspaceAction::TrashObject
        | WorkspaceAction::RestoreObject => ctx.owns_resource,
        WorkspaceAction::CreateShareLink
        | WorkspaceAction::UpdateShareLink
        | WorkspaceAction::RevokeShareLink => ctx.owns_resource && ctx.member_share_links_enabled,
        WorkspaceAction::ViewMembers
        | WorkspaceAction::InviteMember
        | WorkspaceAction::ChangeMemberRole
        | WorkspaceAction::RemoveMember
        | WorkspaceAction::DeleteObjectPermanently
        | WorkspaceAction::ViewShareLinks
        | WorkspaceAction::UpdateWorkspaceSettings
        | WorkspaceAction::ViewBilling
        | WorkspaceAction::ManageBilling
        | WorkspaceAction::ViewAudit
        | WorkspaceAction::ExportAudit
        | WorkspaceAction::ExportWorkspaceData
        | WorkspaceAction::DeleteWorkspace => false,
    }
}

fn viewer_allows(action: WorkspaceAction) -> bool {
    matches!(
        action,
        WorkspaceAction::ViewWorkspace
            | WorkspaceAction::ViewFiles
            | WorkspaceAction::ViewTrash
            | WorkspaceAction::DownloadFile
            | WorkspaceAction::ViewQuota
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authz::action::action_requires_step_up;

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
}
