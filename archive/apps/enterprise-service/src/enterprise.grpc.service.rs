use std::{future::Future, net::SocketAddr};

use tonic::{Request, Response, Status, transport::Server};

use crate::{
    app::EnterpriseAppState,
    grpc::pb::nvbes::{
        enterprise::v1 as enterprise,
        enterprise::v1::enterprise_service_server::{EnterpriseService, EnterpriseServiceServer},
    },
};

#[path = "enterprise.grpc.service.access_reviews.rs"]
mod service_access_reviews;
#[path = "enterprise.grpc.service.admin.rs"]
mod service_admin;
#[path = "enterprise.grpc.service.break_glass.rs"]
mod service_break_glass;
#[path = "enterprise.grpc.service.federation.rs"]
mod service_federation;
#[path = "enterprise.grpc.service.policies.rs"]
mod service_policies;
#[path = "enterprise.grpc.service.user_access.rs"]
mod service_user_access;

#[derive(Clone)]
pub struct EnterpriseGrpcService {
    state: EnterpriseAppState,
}

impl EnterpriseGrpcService {
    pub fn new(state: EnterpriseAppState) -> Self {
        Self { state }
    }
}

pub async fn serve(
    addr: SocketAddr,
    state: EnterpriseAppState,
    authenticator: crate::grpc::auth::EnterpriseGrpcAuthenticator,
    shutdown: impl Future<Output = ()>,
) -> Result<(), tonic::transport::Error> {
    Server::builder()
        .add_service(EnterpriseServiceServer::with_interceptor(
            EnterpriseGrpcService::new(state),
            move |request| authenticator.authenticate(request),
        ))
        .serve_with_shutdown(addr, shutdown)
        .await
}

#[tonic::async_trait]
impl EnterpriseService for EnterpriseGrpcService {
    async fn get_policy_set(
        &self,
        request: Request<enterprise::GetPolicySetRequest>,
    ) -> Result<Response<enterprise::PolicySet>, Status> {
        service_policies::get_policy_set(&self.state.db, request).await
    }

    async fn update_policy(
        &self,
        request: Request<enterprise::UpdatePolicyRequest>,
    ) -> Result<Response<enterprise::EnterprisePolicy>, Status> {
        service_policies::update_policy(&self.state.db, request).await
    }

    async fn evaluate_policy(
        &self,
        request: Request<enterprise::EvaluatePolicyRequest>,
    ) -> Result<Response<enterprise::PolicyDecision>, Status> {
        service_policies::evaluate_policy(&self.state.db, request).await
    }

    async fn authorize_admin_elevation(
        &self,
        request: Request<enterprise::AuthorizeAdminElevationRequest>,
    ) -> Result<Response<enterprise::AdminElevationAuthorization>, Status> {
        service_admin::authorize_admin_elevation(&self.state.db, request).await
    }

    async fn revoke_admin_elevation(
        &self,
        request: Request<enterprise::RevokeAdminElevationRequest>,
    ) -> Result<Response<enterprise::AdminElevationAuthorization>, Status> {
        service_admin::revoke_admin_elevation(&self.state.db, request).await
    }

    async fn approve_privileged_action(
        &self,
        request: Request<enterprise::ApprovePrivilegedActionRequest>,
    ) -> Result<Response<enterprise::PrivilegedActionApproval>, Status> {
        service_admin::approve_privileged_action(&self.state.db, request).await
    }

    async fn consume_privileged_action_approval(
        &self,
        request: Request<enterprise::ConsumePrivilegedActionApprovalRequest>,
    ) -> Result<Response<enterprise::PrivilegedActionApproval>, Status> {
        service_admin::consume_privileged_action_approval(&self.state.db, request).await
    }

    async fn get_trust_center(
        &self,
        request: Request<enterprise::GetTrustCenterRequest>,
    ) -> Result<Response<enterprise::TrustCenter>, Status> {
        service_admin::get_trust_center(&self.state.db, request).await
    }

    async fn list_access_reviews(
        &self,
        request: Request<enterprise::ListAccessReviewsRequest>,
    ) -> Result<Response<enterprise::ListAccessReviewsResponse>, Status> {
        service_access_reviews::list_access_reviews(&self.state.db, request).await
    }

    async fn get_access_review(
        &self,
        request: Request<enterprise::GetAccessReviewRequest>,
    ) -> Result<Response<enterprise::AccessReviewDetail>, Status> {
        service_access_reviews::get_access_review(&self.state.db, request).await
    }

    async fn start_access_review(
        &self,
        request: Request<enterprise::StartAccessReviewRequest>,
    ) -> Result<Response<enterprise::AccessReview>, Status> {
        service_access_reviews::start_access_review(&self.state.db, request).await
    }

    async fn record_access_review_decision(
        &self,
        request: Request<enterprise::RecordAccessReviewDecisionRequest>,
    ) -> Result<Response<enterprise::AccessReviewDecision>, Status> {
        service_access_reviews::record_access_review_decision(&self.state.db, request).await
    }

    async fn close_access_review(
        &self,
        request: Request<enterprise::CloseAccessReviewRequest>,
    ) -> Result<Response<enterprise::AccessReview>, Status> {
        service_access_reviews::close_access_review(&self.state.db, request).await
    }

    async fn list_access_review_schedules(
        &self,
        request: Request<enterprise::ListAccessReviewSchedulesRequest>,
    ) -> Result<Response<enterprise::ListAccessReviewSchedulesResponse>, Status> {
        service_access_reviews::list_access_review_schedules(&self.state.db, request).await
    }

    async fn create_access_review_schedule(
        &self,
        request: Request<enterprise::CreateAccessReviewScheduleRequest>,
    ) -> Result<Response<enterprise::AccessReviewSchedule>, Status> {
        service_access_reviews::create_access_review_schedule(&self.state.db, request).await
    }

    async fn set_access_review_schedule_enabled(
        &self,
        request: Request<enterprise::SetAccessReviewScheduleEnabledRequest>,
    ) -> Result<Response<enterprise::AccessReviewSchedule>, Status> {
        service_access_reviews::set_access_review_schedule_enabled(&self.state.db, request).await
    }

    async fn run_access_review_schedule(
        &self,
        request: Request<enterprise::RunAccessReviewScheduleRequest>,
    ) -> Result<Response<enterprise::AccessReview>, Status> {
        service_access_reviews::run_access_review_schedule(&self.state.db, request).await
    }

    async fn materialize_due_access_review_schedules(
        &self,
        request: Request<enterprise::MaterializeDueAccessReviewSchedulesRequest>,
    ) -> Result<Response<enterprise::AccessReviewScheduleRun>, Status> {
        service_access_reviews::materialize_due_access_review_schedules(&self.state.db, request)
            .await
    }

    async fn claim_access_review_reminder_candidates(
        &self,
        request: Request<enterprise::ClaimAccessReviewReminderCandidatesRequest>,
    ) -> Result<Response<enterprise::AccessReviewReminderClaim>, Status> {
        service_access_reviews::claim_access_review_reminder_candidates(&self.state.db, request)
            .await
    }

    async fn activate_break_glass(
        &self,
        request: Request<enterprise::ActivateBreakGlassRequest>,
    ) -> Result<Response<enterprise::BreakGlassGrant>, Status> {
        service_break_glass::activate_break_glass(&self.state.db, request).await
    }

    async fn revoke_break_glass(
        &self,
        request: Request<enterprise::RevokeBreakGlassRequest>,
    ) -> Result<Response<enterprise::BreakGlassGrant>, Status> {
        service_break_glass::revoke_break_glass(&self.state.db, request).await
    }

    async fn list_break_glass_accounts(
        &self,
        request: Request<enterprise::ListBreakGlassAccountsRequest>,
    ) -> Result<Response<enterprise::ListBreakGlassAccountsResponse>, Status> {
        service_break_glass::list_break_glass_accounts(&self.state.db, request).await
    }

    async fn configure_federation_provider(
        &self,
        request: Request<enterprise::ConfigureFederationProviderRequest>,
    ) -> Result<Response<enterprise::FederationProvider>, Status> {
        service_federation::configure_federation_provider(&self.state.db, request).await
    }

    async fn delete_federation_provider(
        &self,
        request: Request<enterprise::DeleteFederationProviderRequest>,
    ) -> Result<Response<enterprise::FederationProviderDeletion>, Status> {
        service_federation::delete_federation_provider(&self.state.db, request).await
    }

    async fn configure_tenant_domain(
        &self,
        request: Request<enterprise::ConfigureTenantDomainRequest>,
    ) -> Result<Response<enterprise::TenantDomain>, Status> {
        service_federation::configure_tenant_domain(&self.state.db, request).await
    }

    async fn verify_tenant_domain(
        &self,
        request: Request<enterprise::VerifyTenantDomainRequest>,
    ) -> Result<Response<enterprise::TenantDomain>, Status> {
        service_federation::verify_tenant_domain(&self.state.db, request).await
    }

    async fn delete_tenant_domain(
        &self,
        request: Request<enterprise::DeleteTenantDomainRequest>,
    ) -> Result<Response<enterprise::TenantDomainDeletion>, Status> {
        service_federation::delete_tenant_domain(&self.state.db, request).await
    }

    async fn configure_scim_connector(
        &self,
        request: Request<enterprise::ConfigureScimConnectorRequest>,
    ) -> Result<Response<enterprise::ScimConnector>, Status> {
        service_federation::configure_scim_connector(&self.state.db, request).await
    }

    async fn delete_scim_connector(
        &self,
        request: Request<enterprise::DeleteScimConnectorRequest>,
    ) -> Result<Response<enterprise::ScimConnectorDeletion>, Status> {
        service_federation::delete_scim_connector(&self.state.db, request).await
    }

    async fn test_federation_provider(
        &self,
        request: Request<enterprise::TestFederationProviderRequest>,
    ) -> Result<Response<enterprise::FederationTestResult>, Status> {
        service_federation::test_federation_provider(request).await
    }

    async fn get_federation_governance(
        &self,
        request: Request<enterprise::GetFederationGovernanceRequest>,
    ) -> Result<Response<enterprise::FederationGovernance>, Status> {
        service_federation::get_federation_governance(&self.state.db, request).await
    }

    async fn create_invitations(
        &self,
        request: Request<enterprise::CreateInvitationsRequest>,
    ) -> Result<Response<enterprise::CreateInvitationsResponse>, Status> {
        service_admin::create_invitations(&self.state.db, request).await
    }

    async fn record_developer_secret_revoked(
        &self,
        request: Request<enterprise::RecordDeveloperSecretRevokedRequest>,
    ) -> Result<Response<enterprise::EnterpriseAuditRecord>, Status> {
        service_admin::record_developer_secret_revoked(&self.state.db, request).await
    }

    async fn list_audit_events(
        &self,
        request: Request<enterprise::ListAuditEventsRequest>,
    ) -> Result<Response<enterprise::ListAuditEventsResponse>, Status> {
        service_admin::list_audit_events(&self.state.db, request).await
    }

    async fn update_user_access(
        &self,
        request: Request<enterprise::UpdateUserAccessRequest>,
    ) -> Result<Response<enterprise::UserAccessChange>, Status> {
        service_user_access::update_user_access(&self.state.db, request).await
    }

    async fn suspend_user_access(
        &self,
        request: Request<enterprise::SuspendUserAccessRequest>,
    ) -> Result<Response<enterprise::UserAccessChange>, Status> {
        service_user_access::suspend_user_access(&self.state.db, request).await
    }

    async fn reactivate_user_access(
        &self,
        request: Request<enterprise::ReactivateUserAccessRequest>,
    ) -> Result<Response<enterprise::UserAccessChange>, Status> {
        service_user_access::reactivate_user_access(&self.state.db, request).await
    }
}

#[cfg(test)]
#[path = "enterprise.grpc.service.contract_tests.rs"]
mod contract_tests;
