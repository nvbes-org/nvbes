use uuid::Uuid;

use super::campaign_detail;
use crate::domains::enterprise::{
    access_reviews::{
        revocations, runtime,
        types::{
            AccessReviewDecisionInput, AccessReviewDecisionResponse, AccessReviewItemDecision,
            AccessReviewItemType,
        },
    },
    grpc as enterprise_grpc,
};
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

pub async fn decide_item(
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
    tenant_id: Uuid,
    campaign_id: Uuid,
    item_id: Uuid,
    input: AccessReviewDecisionInput,
) -> Result<AccessReviewDecisionResponse, AppError> {
    let decision = enterprise_grpc::access_reviews::record_access_review_decision(
        tenant_id,
        auth.user_id,
        campaign_id,
        item_id,
        &input,
    )
    .await?;

    let mut detail = campaign_detail(tenant_id, auth.user_id, campaign_id).await?;
    let item_index = detail
        .items
        .iter()
        .position(|item| item.id == item_id)
        .ok_or_else(|| {
            AppError::not_found(
                "access_review_item_not_found",
                "Access review item not found.",
            )
        })?;
    let item = detail.items.remove(item_index);
    if matches!(input.decision, AccessReviewItemDecision::Revoked)
        && matches!(item.item_type, AccessReviewItemType::OAuthClient)
    {
        let client = revoked_oauth_client(&decision.target_id, &item.subject_id)?;
        runtime::revoke_oauth_client(redis, &client).await?;
    }
    Ok(AccessReviewDecisionResponse {
        campaign: detail.campaign,
        item,
    })
}

fn revoked_oauth_client(
    target_id: &str,
    client_id: &str,
) -> Result<revocations::RevokedOAuthClient, AppError> {
    let id = Uuid::parse_str(target_id).map_err(|error| {
        AppError::internal(
            "enterprise_grpc_invalid_target_id",
            format!("Enterprise gRPC returned an invalid revocation target ID: {error}"),
        )
    })?;
    Ok(revocations::RevokedOAuthClient {
        id,
        client_id: client_id.to_string(),
    })
}
