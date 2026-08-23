use std::{future::Future, net::SocketAddr};

use tonic::{Request, Response, Status, transport::Server};

use crate::{
    app::AppState,
    grpc::{
        pb::nvbes::{
            cloud::v1 as cloud,
            cloud::v1::cloud_service_server::{CloudService, CloudServiceServer},
        },
        service_workspace,
    },
};

#[derive(Clone)]
pub struct CloudGrpcService {
    state: AppState,
}

impl CloudGrpcService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    pub fn into_server(self) -> CloudServiceServer<Self> {
        CloudServiceServer::new(self)
    }
}

pub async fn serve(
    addr: SocketAddr,
    state: AppState,
    shutdown: impl Future<Output = ()>,
) -> Result<(), tonic::transport::Error> {
    Server::builder()
        .add_service(CloudGrpcService::new(state).into_server())
        .serve_with_shutdown(addr, shutdown)
        .await
}

#[tonic::async_trait]
impl CloudService for CloudGrpcService {
    async fn create_workspace(
        &self,
        request: Request<cloud::CreateWorkspaceRequest>,
    ) -> Result<Response<cloud::Workspace>, Status> {
        let request = request.into_inner();
        let workspace = service_workspace::create_workspace(&self.state.db, request).await?;
        let workspace_id = uuid::Uuid::parse_str(&workspace.workspace_id)
            .map_err(|_| Status::internal("created workspace has an invalid ID"))?;
        self.state.product_analytics.capture(
            nvbes_product_analytics::ProductAnalyticsEvent::workspace(
                "workspace.created",
                workspace_id,
            )
            .property("workspace_type", workspace.workspace_type.clone())
            .property("plan_code", workspace.plan_code.clone()),
        );
        Ok(Response::new(workspace))
    }

    async fn get_workspace(
        &self,
        request: Request<cloud::GetWorkspaceRequest>,
    ) -> Result<Response<cloud::Workspace>, Status> {
        Ok(Response::new(
            service_workspace::get_workspace(&self.state.db, request.into_inner()).await?,
        ))
    }

    async fn update_workspace(
        &self,
        request: Request<cloud::UpdateWorkspaceRequest>,
    ) -> Result<Response<cloud::Workspace>, Status> {
        Ok(Response::new(
            service_workspace::update_workspace(&self.state.db, request.into_inner()).await?,
        ))
    }

    async fn delete_workspace(
        &self,
        request: Request<cloud::DeleteWorkspaceRequest>,
    ) -> Result<Response<cloud::WorkspaceDeletion>, Status> {
        Ok(Response::new(
            service_workspace::delete_workspace(&self.state.db, request.into_inner()).await?,
        ))
    }

    async fn list_workspaces(
        &self,
        request: Request<cloud::ListWorkspacesRequest>,
    ) -> Result<Response<cloud::ListWorkspacesResponse>, Status> {
        Ok(Response::new(
            service_workspace::list_workspaces(&self.state.db, request.into_inner()).await?,
        ))
    }

    async fn add_workspace_member(
        &self,
        request: Request<cloud::AddWorkspaceMemberRequest>,
    ) -> Result<Response<cloud::WorkspaceMember>, Status> {
        Ok(Response::new(
            service_workspace::add_member(&self.state.db, request.into_inner()).await?,
        ))
    }

    async fn update_workspace_member(
        &self,
        request: Request<cloud::UpdateWorkspaceMemberRequest>,
    ) -> Result<Response<cloud::WorkspaceMember>, Status> {
        Ok(Response::new(
            service_workspace::update_member(&self.state.db, request.into_inner()).await?,
        ))
    }

    async fn remove_workspace_member(
        &self,
        request: Request<cloud::RemoveWorkspaceMemberRequest>,
    ) -> Result<Response<cloud::WorkspaceMembershipRemoval>, Status> {
        Ok(Response::new(
            service_workspace::remove_member(&self.state.db, request.into_inner()).await?,
        ))
    }

    async fn list_workspace_members(
        &self,
        request: Request<cloud::ListWorkspaceMembersRequest>,
    ) -> Result<Response<cloud::ListWorkspaceMembersResponse>, Status> {
        Ok(Response::new(
            service_workspace::list_members(&self.state.db, request.into_inner()).await?,
        ))
    }

    async fn create_workspace_invitation(
        &self,
        request: Request<cloud::CreateWorkspaceInvitationRequest>,
    ) -> Result<Response<cloud::WorkspaceInvitation>, Status> {
        Ok(Response::new(
            service_workspace::create_invitation(&self.state.db, request.into_inner()).await?,
        ))
    }

    async fn list_workspace_invitations(
        &self,
        request: Request<cloud::ListWorkspaceInvitationsRequest>,
    ) -> Result<Response<cloud::ListWorkspaceInvitationsResponse>, Status> {
        Ok(Response::new(
            service_workspace::list_invitations(&self.state.db, request.into_inner()).await?,
        ))
    }

    async fn get_workspace_invitation_by_token_hash(
        &self,
        request: Request<cloud::GetWorkspaceInvitationByTokenHashRequest>,
    ) -> Result<Response<cloud::WorkspaceInvitation>, Status> {
        Ok(Response::new(
            service_workspace::get_invitation_by_token_hash(&self.state.db, request.into_inner())
                .await?,
        ))
    }

    async fn accept_workspace_invitation(
        &self,
        request: Request<cloud::AcceptWorkspaceInvitationRequest>,
    ) -> Result<Response<cloud::WorkspaceInvitationAcceptance>, Status> {
        Ok(Response::new(
            service_workspace::accept_invitation(&self.state.db, request.into_inner()).await?,
        ))
    }

    async fn create_folder(
        &self,
        _request: Request<cloud::CreateFolderRequest>,
    ) -> Result<Response<cloud::DriveObject>, Status> {
        Err(unimplemented_storage())
    }

    async fn list_drive_objects(
        &self,
        _request: Request<cloud::ListDriveObjectsRequest>,
    ) -> Result<Response<cloud::ListDriveObjectsResponse>, Status> {
        Err(unimplemented_storage())
    }

    async fn move_drive_object(
        &self,
        _request: Request<cloud::MoveDriveObjectRequest>,
    ) -> Result<Response<cloud::DriveObject>, Status> {
        Err(unimplemented_storage())
    }

    async fn trash_drive_object(
        &self,
        _request: Request<cloud::TrashDriveObjectRequest>,
    ) -> Result<Response<cloud::DriveObject>, Status> {
        Err(unimplemented_storage())
    }

    async fn restore_drive_object(
        &self,
        _request: Request<cloud::RestoreDriveObjectRequest>,
    ) -> Result<Response<cloud::DriveObject>, Status> {
        Err(unimplemented_storage())
    }

    async fn create_upload(
        &self,
        _request: Request<cloud::CreateUploadRequest>,
    ) -> Result<Response<cloud::UploadSession>, Status> {
        Err(unimplemented_storage())
    }

    async fn complete_upload(
        &self,
        _request: Request<cloud::CompleteUploadRequest>,
    ) -> Result<Response<cloud::DriveObject>, Status> {
        Err(unimplemented_storage())
    }

    async fn get_download_url(
        &self,
        _request: Request<cloud::GetDownloadUrlRequest>,
    ) -> Result<Response<cloud::DownloadUrl>, Status> {
        Err(unimplemented_storage())
    }

    async fn get_workspace_quota(
        &self,
        _request: Request<cloud::GetWorkspaceQuotaRequest>,
    ) -> Result<Response<cloud::WorkspaceQuota>, Status> {
        Err(unimplemented_storage())
    }

    async fn reserve_storage(
        &self,
        _request: Request<cloud::ReserveStorageRequest>,
    ) -> Result<Response<cloud::StorageReservation>, Status> {
        Err(unimplemented_storage())
    }

    async fn release_storage(
        &self,
        _request: Request<cloud::ReleaseStorageRequest>,
    ) -> Result<Response<cloud::StorageRelease>, Status> {
        Err(unimplemented_storage())
    }
}

fn unimplemented_storage() -> Status {
    Status::unimplemented("Cloud storage gRPC RPCs are not implemented by cloud-service yet")
}
