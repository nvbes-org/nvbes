use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct ShareLinkListResponse {
    pub share_links: Vec<ShareLinkView>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ShareLinkResponse {
    pub share_link: ShareLinkView,
    pub share_url: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PublicShareResponse {
    pub share: PublicShareView,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PublicDownloadUrlResponse {
    pub share_link_id: Uuid,
    pub download_url: SignedPublicDownloadUrlView,
}

pub struct CreateShareLinkInput {
    pub expires_at: Option<DateTime<Utc>>,
    pub max_downloads: Option<i32>,
}

pub struct UpdateShareLinkInput {
    pub expires_at: Option<DateTime<Utc>>,
    pub max_downloads: Option<Option<i32>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ShareLinkView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub storage_object_id: Uuid,
    pub permission: String,
    pub expires_at: DateTime<Utc>,
    pub max_downloads: Option<i32>,
    pub download_count: i32,
    pub created_by: Uuid,
    pub created_by_principal_id: Uuid,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PublicShareView {
    pub share_link_id: Uuid,
    pub object_id: Uuid,
    pub name: String,
    pub mime_type: Option<String>,
    pub size_bytes: i64,
    pub expires_at: DateTime<Utc>,
    pub remaining_downloads: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SignedPublicDownloadUrlView {
    pub url: String,
    pub method: &'static str,
    pub expires_at: DateTime<Utc>,
}
