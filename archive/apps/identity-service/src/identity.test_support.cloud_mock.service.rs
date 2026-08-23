use super::*;
use cloud::cloud_service_server::CloudService;
use tonic::{Request, Response, Status};

macro_rules! impl_cloud_service {
    ($(($method:ident, $request:ty, $response:ty)),* $(,)?) => {
        #[tonic::async_trait]
        impl CloudService for AccountCloudMock {
            async fn get_workspace(
                &self,
                request: Request<cloud::GetWorkspaceRequest>,
            ) -> Result<Response<cloud::Workspace>, Status> {
                self.get_seeded_workspace(request).await
            }

            async fn list_workspace_members(
                &self,
                request: Request<cloud::ListWorkspaceMembersRequest>,
            ) -> Result<Response<cloud::ListWorkspaceMembersResponse>, Status> {
                self.list_seeded_workspace_members(request).await
            }

            async fn list_workspaces(
                &self,
                request: Request<cloud::ListWorkspacesRequest>,
            ) -> Result<Response<cloud::ListWorkspacesResponse>, Status> {
                self.list_seeded_workspaces(request).await
            }

            $(
                async fn $method(
                    &self,
                    _request: Request<$request>,
                ) -> Result<Response<$response>, Status> {
                    Err(Status::failed_precondition(concat!(
                        "unexpected RPC in OAuth Cloud test server: ",
                        stringify!($method)
                    )))
                }
            )*
        }
    };
}

impl_cloud_service!(
    (
        create_workspace,
        cloud::CreateWorkspaceRequest,
        cloud::Workspace
    ),
    (
        update_workspace,
        cloud::UpdateWorkspaceRequest,
        cloud::Workspace
    ),
    (
        delete_workspace,
        cloud::DeleteWorkspaceRequest,
        cloud::WorkspaceDeletion
    ),
    (
        add_workspace_member,
        cloud::AddWorkspaceMemberRequest,
        cloud::WorkspaceMember
    ),
    (
        update_workspace_member,
        cloud::UpdateWorkspaceMemberRequest,
        cloud::WorkspaceMember
    ),
    (
        remove_workspace_member,
        cloud::RemoveWorkspaceMemberRequest,
        cloud::WorkspaceMembershipRemoval
    ),
    (
        create_workspace_invitation,
        cloud::CreateWorkspaceInvitationRequest,
        cloud::WorkspaceInvitation
    ),
    (
        list_workspace_invitations,
        cloud::ListWorkspaceInvitationsRequest,
        cloud::ListWorkspaceInvitationsResponse
    ),
    (
        get_workspace_invitation_by_token_hash,
        cloud::GetWorkspaceInvitationByTokenHashRequest,
        cloud::WorkspaceInvitation
    ),
    (
        accept_workspace_invitation,
        cloud::AcceptWorkspaceInvitationRequest,
        cloud::WorkspaceInvitationAcceptance
    ),
    (
        create_folder,
        cloud::CreateFolderRequest,
        cloud::DriveObject
    ),
    (
        list_drive_objects,
        cloud::ListDriveObjectsRequest,
        cloud::ListDriveObjectsResponse
    ),
    (
        move_drive_object,
        cloud::MoveDriveObjectRequest,
        cloud::DriveObject
    ),
    (
        trash_drive_object,
        cloud::TrashDriveObjectRequest,
        cloud::DriveObject
    ),
    (
        restore_drive_object,
        cloud::RestoreDriveObjectRequest,
        cloud::DriveObject
    ),
    (
        create_upload,
        cloud::CreateUploadRequest,
        cloud::UploadSession
    ),
    (
        complete_upload,
        cloud::CompleteUploadRequest,
        cloud::DriveObject
    ),
    (
        get_download_url,
        cloud::GetDownloadUrlRequest,
        cloud::DownloadUrl
    ),
    (
        get_workspace_quota,
        cloud::GetWorkspaceQuotaRequest,
        cloud::WorkspaceQuota
    ),
    (
        reserve_storage,
        cloud::ReserveStorageRequest,
        cloud::StorageReservation
    ),
    (
        release_storage,
        cloud::ReleaseStorageRequest,
        cloud::StorageRelease
    ),
);
