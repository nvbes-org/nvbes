use tonic::transport::Channel;
use uuid::Uuid;

use crate::grpc_pb::nvbes::{
    enterprise::v1::{
        AccessReviewReminderClaim, AccessReviewScheduleRun,
        ClaimAccessReviewReminderCandidatesRequest, MaterializeDueAccessReviewSchedulesRequest,
        enterprise_service_client::EnterpriseServiceClient,
    },
    platform::v1::{RequestContext, TenantContext},
};

const ENTERPRISE_GRPC_ENDPOINT_ENV: &str = "NVBES_ENTERPRISE_GRPC_ENDPOINT";
const DEFAULT_ENTERPRISE_GRPC_ENDPOINT: &str = "http://127.0.0.1:4031";
const DUE_SCHEDULE_LIMIT: i32 = 25;
const REMINDER_LIMIT: i32 = 100;

pub async fn materialize_due_access_review_schedules() -> anyhow::Result<AccessReviewScheduleRun> {
    let mut client = enterprise_client().await?;
    let response = client
        .materialize_due_access_review_schedules(MaterializeDueAccessReviewSchedulesRequest {
            context: Some(request_context()),
            limit: DUE_SCHEDULE_LIMIT,
        })
        .await?;
    Ok(response.into_inner())
}

pub async fn claim_access_review_reminder_candidates() -> anyhow::Result<AccessReviewReminderClaim>
{
    let mut client = enterprise_client().await?;
    let response = client
        .claim_access_review_reminder_candidates(ClaimAccessReviewReminderCandidatesRequest {
            context: Some(request_context()),
            limit: REMINDER_LIMIT,
        })
        .await?;
    Ok(response.into_inner())
}

async fn enterprise_client() -> anyhow::Result<EnterpriseServiceClient<Channel>> {
    let endpoint = std::env::var(ENTERPRISE_GRPC_ENDPOINT_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_ENTERPRISE_GRPC_ENDPOINT.to_string());
    Ok(EnterpriseServiceClient::connect(endpoint).await?)
}

fn request_context() -> RequestContext {
    let system_id = Uuid::nil().to_string();
    RequestContext {
        request_id: Uuid::new_v4().to_string(),
        correlation_id: Uuid::new_v4().to_string(),
        actor_principal_id: system_id.clone(),
        tenant: Some(TenantContext {
            tenant_id: system_id,
            workspace_id: String::new(),
            region_id: String::new(),
            data_residency: String::new(),
        }),
    }
}
