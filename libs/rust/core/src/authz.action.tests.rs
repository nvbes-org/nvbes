use super::{
    WorkspaceAction, action_requires_independent_approval, action_requires_step_up, parse_action,
};

#[test]
fn parse_action_accepts_legacy_and_dotted_aliases() {
    let pairs = [
        ("view_workspace", WorkspaceAction::ViewWorkspace),
        ("workspace.view", WorkspaceAction::ViewWorkspace),
        ("files.upload", WorkspaceAction::UploadFile),
        ("billing.manage", WorkspaceAction::ManageBilling),
        ("workspace.delete", WorkspaceAction::DeleteWorkspace),
    ];
    for (raw, expected) in pairs {
        assert_eq!(parse_action(raw), Some(expected), "raw={raw}");
        assert_eq!(expected.as_str(), parse_action(raw).unwrap().as_str());
    }
    assert!(parse_action("not-an-action").is_none());
}

#[test]
fn step_up_required_for_sensitive_mutations() {
    for action in [
        WorkspaceAction::ViewWorkspace,
        WorkspaceAction::UploadFile,
        WorkspaceAction::DownloadFile,
    ] {
        assert!(!action_requires_step_up(action));
    }
    for action in [
        WorkspaceAction::InviteMember,
        WorkspaceAction::ManageBilling,
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
    for action in [
        WorkspaceAction::ExportAudit,
        WorkspaceAction::ExportWorkspaceData,
        WorkspaceAction::ManageKeys,
        WorkspaceAction::DeleteWorkspace,
    ] {
        assert!(action_requires_independent_approval(action));
    }
}
