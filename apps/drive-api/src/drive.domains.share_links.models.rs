#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::domains::files::models::StorageObjectStatus;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "share_link_permission", rename_all = "snake_case")]
pub enum ShareLinkPermission {
    #[serde(rename = "download")]
    Download,
}

#[derive(Debug, Clone, FromRow)]
pub struct ShareLinkRecord {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub storage_object_id: Uuid,
    pub permission: ShareLinkPermission,
    pub expires_at: DateTime<Utc>,
    pub max_downloads: Option<i32>,
    pub download_count: i32,
    pub created_by: Uuid,
    pub created_by_principal_id: Uuid,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SharePolicy {
    pub default_ttl_days: i32,
    pub max_ttl_days: i32,
    pub max_share_links: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct ShareableObjectRecord {
    pub id: Uuid,
    pub status: StorageObjectStatus,
    pub scan_status: String,
}

#[derive(Debug, Clone)]
pub struct PublicShareRecord {
    pub share_link_id: Uuid,
    pub workspace_id: Uuid,
    pub storage_object_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub max_downloads: Option<i32>,
    pub download_count: i32,
    pub revoked_at: Option<DateTime<Utc>>,
    pub name: String,
    pub mime_type: Option<String>,
    pub size_bytes: i64,
    pub object_status: StorageObjectStatus,
    pub scan_status: String,
    pub object_key: Option<String>,
}
