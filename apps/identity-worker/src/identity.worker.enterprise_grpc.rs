use uuid::Uuid;

#[path = "identity.worker.enterprise_grpc.config.rs"]
mod config;

pub use config::EnterpriseGrpcConfig;

use crate::grpc_pb::nvbes::{
    enterprise::v1::{
        AccessReviewReminderClaim, AccessReviewScheduleRun,
        ClaimAccessReviewReminderCandidatesRequest, MaterializeDueAccessReviewSchedulesRequest,
        enterprise_service_client::EnterpriseServiceClient,
    },
    platform::v1::{RequestContext, TenantContext},
};

const DUE_SCHEDULE_LIMIT: i32 = 25;
const REMINDER_LIMIT: i32 = 100;

pub async fn materialize_due_access_review_schedules(
    config: Option<&EnterpriseGrpcConfig>,
) -> anyhow::Result<Option<AccessReviewScheduleRun>> {
    let Some(config) = config else {
        return Ok(None);
    };
    let mut client = EnterpriseServiceClient::new(config.connect().await?);
    let response = client
        .materialize_due_access_review_schedules(config.authorize(
            MaterializeDueAccessReviewSchedulesRequest {
                context: Some(request_context()),
                limit: DUE_SCHEDULE_LIMIT,
            },
        ))
        .await?;
    Ok(Some(response.into_inner()))
}

pub async fn claim_access_review_reminder_candidates(
    config: Option<&EnterpriseGrpcConfig>,
) -> anyhow::Result<Option<AccessReviewReminderClaim>> {
    let Some(config) = config else {
        return Ok(None);
    };
    let mut client = EnterpriseServiceClient::new(config.connect().await?);
    let response = client
        .claim_access_review_reminder_candidates(config.authorize(
            ClaimAccessReviewReminderCandidatesRequest {
                context: Some(request_context()),
                limit: REMINDER_LIMIT,
            },
        ))
        .await?;
    Ok(Some(response.into_inner()))
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

#[cfg(test)]
#[path = "identity.worker.enterprise_grpc.tests.rs"]
mod tests;
