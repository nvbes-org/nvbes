use super::{action::WorkspaceAction, context::ResourceContext, role::WorkspaceRole};

pub fn is_allowed(role: WorkspaceRole, action: WorkspaceAction, ctx: ResourceContext) -> bool {
    match role {
        WorkspaceRole::Owner => owner_allows(action, ctx),
        WorkspaceRole::Admin => admin_allows(action, ctx),
        WorkspaceRole::SecurityAdmin => security_admin_allows(action),
        WorkspaceRole::BillingAdmin => billing_admin_allows(action),
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
                Some(
                    WorkspaceRole::Member
                        | WorkspaceRole::Viewer
                        | WorkspaceRole::SecurityAdmin
                        | WorkspaceRole::BillingAdmin
                )
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

fn security_admin_allows(action: WorkspaceAction) -> bool {
    matches!(
        action,
        WorkspaceAction::ViewWorkspace
            | WorkspaceAction::ViewMembers
            | WorkspaceAction::ViewAudit
            | WorkspaceAction::ExportAudit
            | WorkspaceAction::ViewFiles
            | WorkspaceAction::ViewTrash
            | WorkspaceAction::ViewQuota
    )
}

fn billing_admin_allows(action: WorkspaceAction) -> bool {
    matches!(
        action,
        WorkspaceAction::ViewWorkspace
            | WorkspaceAction::ViewBilling
            | WorkspaceAction::ManageBilling
            | WorkspaceAction::ViewQuota
    )
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
#[path = "authz.policy.tests.rs"]
mod tests;
