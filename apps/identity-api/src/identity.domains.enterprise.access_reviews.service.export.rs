use chrono::Utc;
use uuid::Uuid;

use super::super::types::{AccessReviewCampaignExport, AccessReviewCampaignExportRow};
use super::get_campaign;
use crate::database::Database;
use crate::http::error::AppError;
use crate::http::middleware::jwt::AuthContext;

pub async fn export_campaign(
    db: &Database,
    auth: &AuthContext,
    tenant_id: Uuid,
    campaign_id: Uuid,
) -> Result<AccessReviewCampaignExport, AppError> {
    let detail = get_campaign(db, auth, tenant_id, campaign_id).await?;
    Ok(AccessReviewCampaignExport {
        campaign: detail.campaign,
        generated_at: Utc::now(),
        rows: detail
            .items
            .into_iter()
            .map(|item| AccessReviewCampaignExportRow {
                item_id: item.id,
                item_type: item.item_type,
                subject_id: item.subject_id,
                subject_label: item.subject_label,
                workspace_id: item.workspace_id,
                role: item.role,
                status: item.status,
                decision: item.decision,
                reviewed_by: item.reviewed_by,
                reviewed_at: item.reviewed_at,
                created_at: item.created_at,
                evidence: item.evidence,
            })
            .collect(),
    })
}
