use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceAction {
    ViewWorkspace,
    UpdateWorkspaceSettings,
    ViewMembers,
    InviteMember,
    ChangeMemberRole,
    RemoveMember,
    ViewFiles,
    ViewTrash,
    CreateFolder,
    UploadFile,
    RenameObject,
    MoveObject,
    TrashObject,
    RestoreObject,
    DeleteObjectPermanently,
    DownloadFile,
    ViewShareLinks,
    CreateShareLink,
    UpdateShareLink,
    RevokeShareLink,
    ViewQuota,
    ViewBilling,
    ManageBilling,
    ManageKeys,
    ManageAdministration,
    ViewAudit,
    ExportAudit,
    ExportWorkspaceData,
    DeleteWorkspace,
}

impl WorkspaceAction {
    pub fn as_str(self) -> &'static str {
        match self {
            WorkspaceAction::ViewWorkspace => "view_workspace",
            WorkspaceAction::UpdateWorkspaceSettings => "update_workspace_settings",
            WorkspaceAction::ViewMembers => "view_members",
            WorkspaceAction::InviteMember => "invite_member",
            WorkspaceAction::ChangeMemberRole => "change_member_role",
            WorkspaceAction::RemoveMember => "remove_member",
            WorkspaceAction::ViewFiles => "view_files",
            WorkspaceAction::ViewTrash => "view_trash",
            WorkspaceAction::CreateFolder => "create_folder",
            WorkspaceAction::UploadFile => "upload_file",
            WorkspaceAction::RenameObject => "rename_object",
            WorkspaceAction::MoveObject => "move_object",
            WorkspaceAction::TrashObject => "trash_object",
            WorkspaceAction::RestoreObject => "restore_object",
            WorkspaceAction::DeleteObjectPermanently => "delete_object_permanently",
            WorkspaceAction::DownloadFile => "download_file",
            WorkspaceAction::ViewShareLinks => "view_share_links",
            WorkspaceAction::CreateShareLink => "create_share_link",
            WorkspaceAction::UpdateShareLink => "update_share_link",
            WorkspaceAction::RevokeShareLink => "revoke_share_link",
            WorkspaceAction::ViewQuota => "view_quota",
            WorkspaceAction::ViewBilling => "view_billing",
            WorkspaceAction::ManageBilling => "manage_billing",
            WorkspaceAction::ManageKeys => "manage_keys",
            WorkspaceAction::ManageAdministration => "manage_administration",
            WorkspaceAction::ViewAudit => "view_audit",
            WorkspaceAction::ExportAudit => "export_audit",
            WorkspaceAction::ExportWorkspaceData => "export_workspace_data",
            WorkspaceAction::DeleteWorkspace => "delete_workspace",
        }
    }
}

pub fn action_requires_step_up(action: WorkspaceAction) -> bool {
    matches!(
        action,
        WorkspaceAction::UpdateWorkspaceSettings
            | WorkspaceAction::InviteMember
            | WorkspaceAction::ChangeMemberRole
            | WorkspaceAction::RemoveMember
            | WorkspaceAction::ManageBilling
            | WorkspaceAction::ManageKeys
            | WorkspaceAction::ManageAdministration
            | WorkspaceAction::ExportWorkspaceData
            | WorkspaceAction::DeleteWorkspace
            | WorkspaceAction::DeleteObjectPermanently
    )
}

pub fn parse_action(action: &str) -> Option<WorkspaceAction> {
    match action {
        "view_workspace" | "workspace.view" => Some(WorkspaceAction::ViewWorkspace),
        "update_workspace_settings" | "workspace.update_settings" => {
            Some(WorkspaceAction::UpdateWorkspaceSettings)
        }
        "view_members" | "members.view" => Some(WorkspaceAction::ViewMembers),
        "invite_member" | "members.invite" => Some(WorkspaceAction::InviteMember),
        "change_member_role" | "members.change_role" => Some(WorkspaceAction::ChangeMemberRole),
        "remove_member" | "members.remove" => Some(WorkspaceAction::RemoveMember),
        "view_files" | "files.view" => Some(WorkspaceAction::ViewFiles),
        "view_trash" | "trash.view" => Some(WorkspaceAction::ViewTrash),
        "create_folder" | "files.create_folder" => Some(WorkspaceAction::CreateFolder),
        "upload_file" | "files.upload" => Some(WorkspaceAction::UploadFile),
        "rename_object" | "files.rename" => Some(WorkspaceAction::RenameObject),
        "move_object" | "files.move" => Some(WorkspaceAction::MoveObject),
        "trash_object" | "files.trash" => Some(WorkspaceAction::TrashObject),
        "restore_object" | "files.restore" => Some(WorkspaceAction::RestoreObject),
        "delete_object_permanently" | "files.delete_permanently" => {
            Some(WorkspaceAction::DeleteObjectPermanently)
        }
        "download_file" | "files.download" => Some(WorkspaceAction::DownloadFile),
        "view_share_links" | "share_links.view" => Some(WorkspaceAction::ViewShareLinks),
        "create_share_link" | "share_links.create" => Some(WorkspaceAction::CreateShareLink),
        "update_share_link" | "share_links.update" => Some(WorkspaceAction::UpdateShareLink),
        "revoke_share_link" | "share_links.revoke" => Some(WorkspaceAction::RevokeShareLink),
        "view_quota" | "quota.view" => Some(WorkspaceAction::ViewQuota),
        "view_billing" | "billing.view" => Some(WorkspaceAction::ViewBilling),
        "manage_billing" | "billing.manage" => Some(WorkspaceAction::ManageBilling),
        "manage_keys" | "keys.manage" => Some(WorkspaceAction::ManageKeys),
        "manage_administration" | "administration.manage" => {
            Some(WorkspaceAction::ManageAdministration)
        }
        "view_audit" | "audit.view" => Some(WorkspaceAction::ViewAudit),
        "export_audit" | "audit.export" => Some(WorkspaceAction::ExportAudit),
        "export_workspace_data" | "workspace.export_data" => {
            Some(WorkspaceAction::ExportWorkspaceData)
        }
        "delete_workspace" | "workspace.delete" => Some(WorkspaceAction::DeleteWorkspace),
        _ => None,
    }
}

pub fn action_requires_independent_approval(action: WorkspaceAction) -> bool {
    matches!(
        action,
        WorkspaceAction::ManageBilling
            | WorkspaceAction::ManageKeys
            | WorkspaceAction::ManageAdministration
            | WorkspaceAction::ExportAudit
            | WorkspaceAction::ExportWorkspaceData
            | WorkspaceAction::DeleteWorkspace
    )
}
