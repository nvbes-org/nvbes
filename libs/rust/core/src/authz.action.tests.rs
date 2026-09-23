use super::{
    WorkspaceAction, action_requires_independent_approval, action_requires_step_up, parse_action,
};

#[test]
fn parse_action_accepts_legacy_and_dotted_aliases() {
    let pairs = [
        ("view_workspace", WorkspaceAction::ViewWorkspace),
        ("workspace.view", WorkspaceAction::ViewWorkspace),
        (
            "update_workspace_settings",
            WorkspaceAction::UpdateWorkspaceSettings,
        ),
        (
            "workspace.update_settings",
            WorkspaceAction::UpdateWorkspaceSettings,
        ),
        ("view_members", WorkspaceAction::ViewMembers),
        ("members.view", WorkspaceAction::ViewMembers),
        ("invite_member", WorkspaceAction::InviteMember),
        ("members.invite", WorkspaceAction::InviteMember),
        ("change_member_role", WorkspaceAction::ChangeMemberRole),
        ("members.change_role", WorkspaceAction::ChangeMemberRole),
        ("remove_member", WorkspaceAction::RemoveMember),
        ("members.remove", WorkspaceAction::RemoveMember),
        ("view_files", WorkspaceAction::ViewFiles),
        ("files.view", WorkspaceAction::ViewFiles),
        ("view_trash", WorkspaceAction::ViewTrash),
        ("trash.view", WorkspaceAction::ViewTrash),
        ("create_folder", WorkspaceAction::CreateFolder),
        ("files.create_folder", WorkspaceAction::CreateFolder),
        ("upload_file", WorkspaceAction::UploadFile),
        ("files.upload", WorkspaceAction::UploadFile),
        ("rename_object", WorkspaceAction::RenameObject),
        ("files.rename", WorkspaceAction::RenameObject),
        ("move_object", WorkspaceAction::MoveObject),
        ("files.move", WorkspaceAction::MoveObject),
        ("trash_object", WorkspaceAction::TrashObject),
        ("files.trash", WorkspaceAction::TrashObject),
        ("restore_object", WorkspaceAction::RestoreObject),
        ("files.restore", WorkspaceAction::RestoreObject),
        (
            "delete_object_permanently",
            WorkspaceAction::DeleteObjectPermanently,
        ),
        (
            "files.delete_permanently",
            WorkspaceAction::DeleteObjectPermanently,
        ),
        ("download_file", WorkspaceAction::DownloadFile),
        ("files.download", WorkspaceAction::DownloadFile),
        ("view_share_links", WorkspaceAction::ViewShareLinks),
        ("share_links.view", WorkspaceAction::ViewShareLinks),
        ("create_share_link", WorkspaceAction::CreateShareLink),
        ("share_links.create", WorkspaceAction::CreateShareLink),
        ("update_share_link", WorkspaceAction::UpdateShareLink),
        ("share_links.update", WorkspaceAction::UpdateShareLink),
        ("revoke_share_link", WorkspaceAction::RevokeShareLink),
        ("share_links.revoke", WorkspaceAction::RevokeShareLink),
        ("view_quota", WorkspaceAction::ViewQuota),
        ("quota.view", WorkspaceAction::ViewQuota),
        ("view_billing", WorkspaceAction::ViewBilling),
        ("billing.view", WorkspaceAction::ViewBilling),
        ("manage_billing", WorkspaceAction::ManageBilling),
        ("billing.manage", WorkspaceAction::ManageBilling),
        ("manage_keys", WorkspaceAction::ManageKeys),
        ("keys.manage", WorkspaceAction::ManageKeys),
        (
            "manage_administration",
            WorkspaceAction::ManageAdministration,
        ),
        (
            "administration.manage",
            WorkspaceAction::ManageAdministration,
        ),
        ("view_audit", WorkspaceAction::ViewAudit),
        ("audit.view", WorkspaceAction::ViewAudit),
        ("export_audit", WorkspaceAction::ExportAudit),
        ("audit.export", WorkspaceAction::ExportAudit),
        (
            "export_workspace_data",
            WorkspaceAction::ExportWorkspaceData,
        ),
        (
            "workspace.export_data",
            WorkspaceAction::ExportWorkspaceData,
        ),
        ("delete_workspace", WorkspaceAction::DeleteWorkspace),
        ("workspace.delete", WorkspaceAction::DeleteWorkspace),
    ];
    for (raw, expected) in pairs {
        assert_eq!(parse_action(raw), Some(expected), "raw={raw}");
        assert_eq!(expected.as_str(), parse_action(raw).unwrap().as_str());
    }
    assert!(parse_action("not-an-action").is_none());
}

#[test]
fn as_str_covers_every_workspace_action_variant() {
    let expected = [
        (WorkspaceAction::ViewWorkspace, "view_workspace"),
        (
            WorkspaceAction::UpdateWorkspaceSettings,
            "update_workspace_settings",
        ),
        (WorkspaceAction::ViewMembers, "view_members"),
        (WorkspaceAction::InviteMember, "invite_member"),
        (WorkspaceAction::ChangeMemberRole, "change_member_role"),
        (WorkspaceAction::RemoveMember, "remove_member"),
        (WorkspaceAction::ViewFiles, "view_files"),
        (WorkspaceAction::ViewTrash, "view_trash"),
        (WorkspaceAction::CreateFolder, "create_folder"),
        (WorkspaceAction::UploadFile, "upload_file"),
        (WorkspaceAction::RenameObject, "rename_object"),
        (WorkspaceAction::MoveObject, "move_object"),
        (WorkspaceAction::TrashObject, "trash_object"),
        (WorkspaceAction::RestoreObject, "restore_object"),
        (
            WorkspaceAction::DeleteObjectPermanently,
            "delete_object_permanently",
        ),
        (WorkspaceAction::DownloadFile, "download_file"),
        (WorkspaceAction::ViewShareLinks, "view_share_links"),
        (WorkspaceAction::CreateShareLink, "create_share_link"),
        (WorkspaceAction::UpdateShareLink, "update_share_link"),
        (WorkspaceAction::RevokeShareLink, "revoke_share_link"),
        (WorkspaceAction::ViewQuota, "view_quota"),
        (WorkspaceAction::ViewBilling, "view_billing"),
        (WorkspaceAction::ManageBilling, "manage_billing"),
        (WorkspaceAction::ManageKeys, "manage_keys"),
        (
            WorkspaceAction::ManageAdministration,
            "manage_administration",
        ),
        (WorkspaceAction::ViewAudit, "view_audit"),
        (WorkspaceAction::ExportAudit, "export_audit"),
        (
            WorkspaceAction::ExportWorkspaceData,
            "export_workspace_data",
        ),
        (WorkspaceAction::DeleteWorkspace, "delete_workspace"),
    ];
    for (action, label) in expected {
        assert_eq!(action.as_str(), label);
    }
}

#[test]
fn step_up_required_for_sensitive_mutations() {
    for action in [
        WorkspaceAction::ViewWorkspace,
        WorkspaceAction::UploadFile,
        WorkspaceAction::DownloadFile,
        WorkspaceAction::ViewAudit,
        WorkspaceAction::ExportAudit,
    ] {
        assert!(!action_requires_step_up(action));
    }
    for action in [
        WorkspaceAction::UpdateWorkspaceSettings,
        WorkspaceAction::InviteMember,
        WorkspaceAction::ChangeMemberRole,
        WorkspaceAction::RemoveMember,
        WorkspaceAction::ManageBilling,
        WorkspaceAction::ManageKeys,
        WorkspaceAction::ManageAdministration,
        WorkspaceAction::ExportWorkspaceData,
        WorkspaceAction::DeleteWorkspace,
        WorkspaceAction::DeleteObjectPermanently,
    ] {
        assert!(action_requires_step_up(action));
    }
}

#[test]
fn independent_approval_required_for_high_risk_exports() {
    assert!(!action_requires_independent_approval(
        WorkspaceAction::ViewAudit
    ));
    assert!(!action_requires_independent_approval(
        WorkspaceAction::InviteMember
    ));
    for action in [
        WorkspaceAction::ManageBilling,
        WorkspaceAction::ManageKeys,
        WorkspaceAction::ManageAdministration,
        WorkspaceAction::ExportAudit,
        WorkspaceAction::ExportWorkspaceData,
        WorkspaceAction::DeleteWorkspace,
    ] {
        assert!(action_requires_independent_approval(action));
    }
}
